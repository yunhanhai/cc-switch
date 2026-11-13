//! Incremental, bounded decoding for native Codex Responses inspection.

use super::{content_encoding::get_content_encoding, hyper_client::ProxyResponse};
use async_compression::tokio::bufread::{
    BrotliDecoder, DeflateDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder,
};
use bytes::Bytes;
use std::{io, pin::Pin};
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio_util::io::StreamReader;

const MAX_CODING_LAYERS: usize = 4;
const READ_CHUNK_BYTES: usize = 8 * 1024;
// Limit decoder memory independently of the total output budget (128 MiB).
const ZSTD_MAX_WINDOW_LOG: u32 = 23;

type Reader = Pin<Box<dyn AsyncRead + Send>>;

#[derive(Debug)]
struct ByteBudgetExceeded(usize);

impl std::fmt::Display for ByteBudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Responses stream exceeds its {} byte budget", self.0)
    }
}

impl std::error::Error for ByteBudgetExceeded {}

pub(crate) fn exceeded_byte_budget(error: &io::Error) -> Option<usize> {
    error
        .get_ref()
        .and_then(|error| error.downcast_ref::<ByteBudgetExceeded>())
        .map(|error| error.0)
}

/// Unsupported encodings and excessive stacks keep their original body and
/// headers. Each decoded stage, including intermediate layers, has a budget.
/// Dropping this response drops every decoder and the upstream stream.
pub(crate) fn decode_response(response: ProxyResponse, max_bytes: usize) -> ProxyResponse {
    let Some(encoding) = get_content_encoding(response.headers()) else {
        return response;
    };
    let codings: Vec<String> = encoding
        .split(',')
        .map(str::trim)
        .filter(|coding| !coding.is_empty() && *coding != "identity")
        .map(str::to_owned)
        .collect();
    if codings.len() > MAX_CODING_LAYERS
        || !codings.iter().all(|coding| {
            matches!(
                coding.as_str(),
                "gzip" | "x-gzip" | "deflate" | "br" | "zstd" | "zst"
            )
        })
    {
        return response;
    }

    let status = response.status();
    let mut headers = response.headers().clone();
    // These describe the encoded representation. Preserve status, content type,
    // retry guidance and request IDs; Hyper supplies fresh transfer framing.
    for name in [
        "content-encoding",
        "content-length",
        "transfer-encoding",
        "content-md5",
        "digest",
        "content-digest",
        "etag",
        "accept-ranges",
    ] {
        headers.remove(name);
    }

    let decoded = async_stream::try_stream! {
        let upstream = Box::pin(response.bytes_stream());
        let mut reader: Reader = limited_reader(
            Box::pin(StreamReader::new(upstream)),
            super::hyper_client::MAX_RESPONSE_BODY_BYTES,
        );
        for coding in codings.into_iter().rev() {
            let buffered = BufReader::with_capacity(READ_CHUNK_BYTES, reader);
            reader = match coding.as_str() {
                "gzip" | "x-gzip" => {
                    let mut decoder = GzipDecoder::new(buffered);
                    decoder.multiple_members(true);
                    Box::pin(decoder)
                }
                "deflate" => deflate_reader(buffered).await?,
                "br" => Box::pin(BrotliDecoder::new(buffered)),
                "zstd" | "zst" => {
                    let mut decoder = ZstdDecoder::with_params(
                        buffered,
                        &[async_compression::codecs::zstd::params::DParameter::window_log_max(
                            ZSTD_MAX_WINDOW_LOG,
                        )],
                    );
                    decoder.multiple_members(true);
                    Box::pin(decoder)
                }
                _ => unreachable!("content codings were validated before consuming the body"),
            };
            reader = limited_reader(reader, max_bytes);
        }
        let mut chunk = vec![0; READ_CHUNK_BYTES];
        loop {
            let count = reader.read(&mut chunk).await?;
            if count == 0 {
                break;
            }
            yield Bytes::copy_from_slice(&chunk[..count]);
        }
    };
    ProxyResponse::streamed(status, headers, decoded)
}

/// Read at most one byte beyond the budget, then fail instead of signalling a
/// clean EOF. A decoder may otherwise interpret a truncated compressed stage
/// as a complete body and hide an output limit violation.
fn limited_reader(mut reader: Reader, limit: usize) -> Reader {
    let stream = async_stream::try_stream! {
        let mut remaining = limit;
        let mut chunk = vec![0; READ_CHUNK_BYTES];
        loop {
            let requested = READ_CHUNK_BYTES.min(remaining.saturating_add(1));
            let count = reader.read(&mut chunk[..requested]).await?;
            if count == 0 {
                break;
            }
            if count > remaining {
                Err(io::Error::other(ByteBudgetExceeded(limit)))?;
            }
            remaining -= count;
            yield Bytes::copy_from_slice(&chunk[..count]);
        }
    };
    let stream: Pin<Box<dyn futures::Stream<Item = io::Result<Bytes>> + Send>> = Box::pin(stream);
    Box::pin(StreamReader::new(stream))
}

/// HTTP deflate is normally zlib-wrapped; some gateways send raw DEFLATE.
/// Sniff only the fixed two-byte zlib header, retaining both bytes. Rewinding
/// after a decode failure would require buffering the entire live stream.
async fn deflate_reader(mut reader: BufReader<Reader>) -> io::Result<Reader> {
    let mut prefix = [0; 2];
    reader.read_exact(&mut prefix).await?;
    let zlib = prefix[0] & 0x0f == 8 && prefix[0] >> 4 <= 7 && u16::from_be_bytes(prefix) % 31 == 0;
    let replay = BufReader::with_capacity(READ_CHUNK_BYTES, io::Cursor::new(prefix).chain(reader));
    Ok(if zlib {
        Box::pin(ZlibDecoder::new(replay))
    } else {
        Box::pin(DeflateDecoder::new(replay))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use http::{HeaderMap, StatusCode};
    use std::io::Write;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;

    fn compress(coding: &str, payload: &[u8]) -> Vec<u8> {
        match coding {
            "gzip" | "x-gzip" => {
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(payload).unwrap();
                encoder.finish().unwrap()
            }
            "deflate" => {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(payload).unwrap();
                encoder.finish().unwrap()
            }
            "raw-deflate" => {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(payload).unwrap();
                encoder.finish().unwrap()
            }
            "br" => {
                let mut output = Vec::new();
                {
                    let mut writer = brotli::CompressorWriter::new(&mut output, 4096, 5, 22);
                    writer.write_all(payload).unwrap();
                }
                output
            }
            "zstd" | "zst" => zstd::stream::encode_all(io::Cursor::new(payload), 1).unwrap(),
            _ => unreachable!(),
        }
    }

    fn response(coding: &str, encoded: &[u8]) -> ProxyResponse {
        let mut headers = HeaderMap::new();
        for (name, value) in [
            ("content-encoding", coding),
            ("content-type", "text/event-stream"),
            ("content-length", "999"),
            ("transfer-encoding", "chunked"),
            ("digest", "synthetic-digest"),
            ("retry-after", "2"),
            ("x-request-id", "synthetic-request"),
        ] {
            headers.insert(
                http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        let chunks: Vec<_> = encoded
            .chunks(1)
            .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
            .collect();
        ProxyResponse::streamed(StatusCode::OK, headers, futures::stream::iter(chunks))
    }

    #[tokio::test]
    async fn supported_codings_decode_across_single_byte_chunks_and_update_headers() {
        let payload =
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"你好\"}\n\n".as_bytes();
        for coding in [
            "gzip",
            "x-gzip",
            "deflate",
            "raw-deflate",
            "br",
            "zstd",
            "zst",
        ] {
            let encoded = compress(coding, payload);
            let header = if coding == "raw-deflate" {
                "deflate"
            } else {
                coding
            };
            let decoded = decode_response(response(header, &encoded), 1024);
            for name in [
                "content-encoding",
                "content-length",
                "transfer-encoding",
                "digest",
            ] {
                assert!(
                    !decoded.headers().contains_key(name),
                    "{coding}: stale {name}"
                );
            }
            assert_eq!(decoded.status(), StatusCode::OK);
            assert_eq!(decoded.headers()["retry-after"], "2");
            assert_eq!(decoded.headers()["x-request-id"], "synthetic-request");
            assert_eq!(
                decoded.bytes_with_limit(1024).await.unwrap().as_ref(),
                payload,
                "{coding}"
            );
        }
    }

    #[tokio::test]
    async fn stacked_and_concatenated_members_preserve_all_decoded_bytes() {
        let payload = b"data: synthetic event\n\n";
        let stacked = compress("zstd", &compress("gzip", payload));
        let mut upstream = response("gzip", &stacked);
        if let ProxyResponse::Streamed { headers, .. } = &mut upstream {
            headers.append("content-encoding", "identity, zstd".parse().unwrap());
        }
        assert_eq!(
            decode_response(upstream, 1024)
                .bytes_with_limit(1024)
                .await
                .unwrap()
                .as_ref(),
            payload
        );
        for coding in ["gzip", "zstd"] {
            let encoded = [compress(coding, b"first"), compress(coding, b"second")].concat();
            assert_eq!(
                decode_response(response(coding, &encoded), 1024)
                    .bytes_with_limit(1024)
                    .await
                    .unwrap()
                    .as_ref(),
                b"firstsecond",
                "{coding}"
            );
        }
    }

    #[tokio::test]
    async fn unknown_and_excessive_codings_keep_the_encoded_representation() {
        let payload = b"opaque compressed bytes";
        for coding in ["snappy", "gzip, unknown", "gzip, gzip, gzip, gzip, gzip"] {
            let encoded = response(coding, payload);
            let original = encoded.headers().clone();
            let decoded = decode_response(encoded, 128);
            assert_eq!(decoded.headers(), &original);
            assert_eq!(
                decoded.bytes_with_limit(128).await.unwrap().as_ref(),
                payload
            );
        }
    }

    #[tokio::test]
    async fn corrupt_and_truncated_streams_report_errors_instead_of_clean_eof() {
        for coding in ["gzip", "deflate", "br", "zstd"] {
            let mut encoded = compress(coding, &b"plain payload ".repeat(100));
            encoded.truncate(encoded.len() / 2);
            assert!(
                decode_response(response(coding, &encoded), 4096)
                    .bytes_with_limit(4096)
                    .await
                    .is_err(),
                "{coding}: truncated input accepted"
            );
            assert!(
                decode_response(response(coding, b"not a compressed frame"), 4096)
                    .bytes_with_limit(4096)
                    .await
                    .is_err(),
                "{coding}: corrupt input accepted"
            );
        }
    }

    #[tokio::test]
    async fn output_and_intermediate_budgets_abort_excessive_decompression() {
        for coding in ["gzip", "deflate", "br", "zstd"] {
            let encoded = compress(coding, &vec![b'a'; 16 * 1024]);
            assert!(
                decode_response(response(coding, &encoded), 128)
                    .bytes_with_limit(64 * 1024)
                    .await
                    .is_err(),
                "{coding}: output limit ignored"
            );
        }
        let payload = b"small final output";
        // A gzip filename makes the intermediate gzip body exceed the budget,
        // although its final output is small. The outer layer must still stop.
        let mut gzip = flate2::GzBuilder::new()
            .filename(vec![b'x'; 2048])
            .write(Vec::new(), flate2::Compression::default());
        gzip.write_all(payload).unwrap();
        let stacked = compress("zstd", &gzip.finish().unwrap());
        assert!(decode_response(response("gzip, zstd", &stacked), 128)
            .bytes_with_limit(4096)
            .await
            .is_err());
        let exactly = compress("gzip", payload);
        assert_eq!(
            decode_response(response("gzip", &exactly), payload.len())
                .bytes_with_limit(4096)
                .await
                .unwrap()
                .as_ref(),
            payload
        );
    }

    #[tokio::test]
    async fn first_decoded_output_arrives_before_upstream_eof_and_drop_cancels_upstream() {
        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let owned = DropFlag(dropped.clone());
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"first decoded output").unwrap();
        encoder.flush().unwrap();
        let prefix = Bytes::copy_from_slice(encoder.get_ref());
        let upstream = async_stream::stream! {
            let _owned = owned;
            yield Ok(prefix);
            std::future::pending::<()>().await;
        };
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip".parse().unwrap());
        let mut decoded = Box::pin(
            decode_response(
                ProxyResponse::streamed(StatusCode::OK, headers, upstream),
                4096,
            )
            .bytes_stream(),
        );
        let output = tokio::time::timeout(Duration::from_secs(1), decoded.next())
            .await
            .expect("must not wait for EOF")
            .unwrap()
            .unwrap();
        assert_eq!(output.as_ref(), b"first decoded output");
        assert!(!dropped.load(Ordering::SeqCst));
        drop(decoded);
        assert!(
            dropped.load(Ordering::SeqCst),
            "decoding detached the upstream stream"
        );
    }

    #[tokio::test]
    async fn decoder_limit_reports_a_non_retryable_byte_budget_error() {
        let encoded = compress("gzip", &vec![b'a'; 4096]);
        let mut decoded = Box::pin(decode_response(response("gzip", &encoded), 128).bytes_stream());
        let error = decoded.next().await.unwrap().unwrap_err();
        assert_eq!(exceeded_byte_budget(&error), Some(128));
    }

    #[tokio::test]
    async fn zstd_window_limit_applies_to_later_concatenated_members() {
        let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 1).unwrap();
        encoder.window_log(ZSTD_MAX_WINDOW_LOG + 1).unwrap();
        encoder
            .write_all(b"small output with an excessive advertised window")
            .unwrap();
        let oversized = encoder.finish().unwrap();
        // The synchronous default decoder confirms the fixture is valid.
        assert_eq!(
            zstd::stream::decode_all(io::Cursor::new(&oversized)).unwrap(),
            b"small output with an excessive advertised window"
        );
        for encoded in [
            oversized.clone(),
            [compress("zstd", b"first"), oversized].concat(),
        ] {
            assert!(
                decode_response(response("zstd", &encoded), 4096)
                    .bytes_with_limit(4096)
                    .await
                    .is_err(),
                "zstd window budget was not enforced"
            );
        }
    }
}

// Called only after windows-release-smoke.mjs validates a disposable runner.
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { connect } from "node:net";
import { gzipSync, brotliCompressSync } from "node:zlib";

async function openCompatibilitySettings(cdp, until) {
  await until(
    () =>
      cdp.evaluate(`(() => {
        if (document.querySelector('[role="tab"][id$="-trigger-proxy"]')) return true;
        const button = [...document.querySelectorAll('button[title]')]
          .find(item => /^(Settings|设置)$/.test(item.title));
        if (!button) return false;
        button.click();
        return true;
      })()`),
    "open settings",
  );
  await until(
    () =>
      cdp.evaluate(`(() => {
        const tab = document.querySelector('[role="tab"][id$="-trigger-proxy"]');
        if (!tab) return false;
        if (tab.getAttribute('aria-selected') === 'true') return true;
        // Radix Tabs activates on mousedown or keyboard input, not click().
        tab.dispatchEvent(new MouseEvent('mousedown', {bubbles: true, button: 0}));
        return false;
      })()`),
    "open routing tab",
  );
  await until(
    () =>
      cdp.evaluate(`(() => {
        if (document.getElementById('codex-responses-compatibility')) return true;
        const trigger = [...document.querySelectorAll('button[aria-expanded]')]
          .find(item => /Rectifier|整流器/.test(item.innerText));
        if (!trigger) return false;
        if (trigger.getAttribute('aria-expanded') === 'false') trigger.click();
        return false;
      })()`),
    "render Codex compatibility switch",
  );
}

async function setCompatibility(cdp, until, enabled) {
  const state = enabled ? "true" : "false";
  assert.equal(
    await cdp.evaluate(`(() => {
      const control = document.getElementById('codex-responses-compatibility');
      if (!control || control.disabled) return false;
      control.scrollIntoView({block: 'center'});
      if (control.getAttribute('aria-checked') !== '${state}') control.click();
      return true;
    })()`),
    true,
  );
  await until(
    async () =>
      (await cdp.invoke("get_rectifier_config")).codexResponsesCompatibility ===
      enabled,
    `save compatibility ${enabled}`,
  );
  assert.equal(
    await cdp.evaluate(
      "document.getElementById('codex-responses-compatibility').getAttribute('aria-checked')",
    ),
    state,
  );
}

export async function exerciseCodexCompatibility(cdp, { check, until, label }) {
  const requests = [];
  let scenario = "history";
  let hits = 0;
  let releaseEarlierSuccess;
  function sendStream(response, text) {
    const coding = scenario.includes("gzip")
      ? "gzip"
      : scenario.includes("brotli")
        ? "br"
        : null;
    const data =
      coding === "gzip"
        ? gzipSync(text)
        : coding === "br"
          ? brotliCompressSync(text)
          : Buffer.from(text);
    response.writeHead(200, {
      "content-type": "text/event-stream",
      ...(coding ? { "content-encoding": coding } : {}),
    });
    for (let offset = 0; offset < data.length; offset += 7)
      response.write(data.subarray(offset, offset + 7));
    response.end();
  }
  const ciphertext = "synthetic-ciphertext-from-another-pool-resource";
  const upstream = createServer(async (request, response) => {
    try {
      const chunks = [];
      for await (const chunk of request) chunks.push(chunk);
      const body = JSON.parse(Buffer.concat(chunks).toString());
      requests.push({
        path: request.url,
        authorization: request.headers.authorization,
        body,
      });
      hits += 1;
      if (scenario === "stateful-503") {
        response.writeHead(503, { "content-type": "application/json" });
        response.end(JSON.stringify({ error: { code: "server_error" } }));
        return;
      }
      if (scenario === "quota-429") {
        response.writeHead(429, {
          "content-type": "application/json",
          "retry-after": "1",
        });
        response.end(
          JSON.stringify({
            error: { type: "rate_limit_error", code: "insufficient_quota" },
          }),
        );
        return;
      }
      if (scenario === "body-wait-hint" || scenario === "repeated-wait-hint") {
        response.writeHead(429, {
          "content-type": "application/json",
          ...(scenario === "repeated-wait-hint"
            ? { "retry-after": ["bad", "11", "23"] }
            : {}),
        });
        response.end(
          JSON.stringify({
            error: {
              code: "rate_limit_exceeded",
              retry_after_seconds: null,
              retry_after: "17",
            },
          }),
        );
        return;
      }
      if (scenario === "cooldown-race") {
        if (body.input === "Earlier success") {
          await new Promise((done) => {
            releaseEarlierSuccess = done;
          });
        } else if (body.input === "Newer rate limit") {
          response.writeHead(429, {
            "content-type": "application/json",
            "retry-after": "30",
          });
          response.end(
            JSON.stringify({ error: { code: "rate_limit_exceeded" } }),
          );
          return;
        }
      }
      if (
        scenario === "always-429" ||
        (scenario === "cancel-wait" && body.input === "Cancelled retry") ||
        (scenario === "retry-429" && hits === 1)
      ) {
        response.writeHead(429, {
          "content-type": "application/json",
          "retry-after": scenario === "cancel-wait" ? "5" : "1",
        });
        response.end(
          JSON.stringify({
            error: {
              type: "rate_limit_error",
              message: "Synthetic rate limit",
            },
          }),
        );
        return;
      }
      const streamFailure =
        scenario === "always-stream-429" ||
        scenario === "always-gzip-stream-429" ||
        ((scenario === "retry-stream-429" ||
          scenario === "retry-stream-503" ||
          scenario === "retry-gzip-stream-429" ||
          scenario === "retry-brotli-stream-429") &&
          hits === 1);
      if (
        streamFailure ||
        scenario === "late-stream-error" ||
        scenario === "late-gzip-stream-error"
      ) {
        const code =
          scenario === "retry-stream-503" || scenario.includes("late-")
            ? "server_error"
            : "rate_limit_exceeded";
        let text =
          'event: response.created\ndata: {"type":"response.created","response":{"status":"in_progress","output":[]}}\n\n';
        if (scenario.includes("late-")) {
          text +=
            'event: response.output_text.delta\ndata: {"type":"response.output_text.delta","delta":"only-once"}\n\n';
        }
        text +=
          "event: response.failed\ndata: " +
          JSON.stringify({
            type: "response.failed",
            response: {
              status: "failed",
              error: { code, retry_after_seconds: 1 },
            },
          }) +
          "\n\n";
        sendStream(response, text);
        return;
      }
      if (JSON.stringify(body).includes(ciphertext)) {
        response.writeHead(200, { "content-type": "text/event-stream" });
        response.end(
          'event: response.failed\ndata: {"type":"response.failed","response":{"id":"resp_ci_failed","status":"failed","error":{"code":"invalid_encrypted_content","message":"Synthetic pool cannot decrypt history"}}}\n\n',
        );
        return;
      }
      const result = {
        id: "resp_ci_compatible",
        object: "response",
        status: "completed",
        model: body.model,
        output: [
          {
            type: "message",
            id: "msg_ci_result",
            role: "assistant",
            status: "completed",
            content: [{ type: "output_text", text: "YunSwitch Codex smoke" }],
          },
        ],
        usage: { input_tokens: 1, output_tokens: 4, total_tokens: 5 },
      };
      if (body.stream) {
        sendStream(
          response,
          `event: response.completed\ndata: ${JSON.stringify({ type: "response.completed", response: result })}\n\n`,
        );
      } else {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify(result));
      }
    } catch (error) {
      response.writeHead(500, { "content-type": "application/json" });
      response.end(JSON.stringify({ error: String(error) }));
    }
  });
  await new Promise((done, reject) =>
    upstream.once("error", reject).listen(0, "127.0.0.1", done),
  );
  let proxyStarted = false;
  const proxyBefore = await cdp.invoke("get_proxy_config_for_app", {
    appType: "codex",
  });
  const rectifierBefore = await cdp.invoke("get_rectifier_config");
  assert.equal(rectifierBefore.codexResponsesCompatibility, false);
  const base = `http://127.0.0.1:${upstream.address().port}/v1`;
  const provider = {
    id: "yunswitch-ci-codex",
    name: "CI synthetic Codex gateway",
    category: "custom",
    settingsConfig: {
      base_url: base,
      auth: { OPENAI_API_KEY: "fake-yunswitch-codex-key" },
      config: `model = "ci-model"\nmodel_provider = "ci_gateway"\n[model_providers.ci_gateway]\nname = "CI Gateway"\nbase_url = "${base}"\nwire_api = "responses"\nrequires_openai_auth = true\n`,
    },
  };
  const history = {
    model: "ci-model",
    stream: true,
    store: false,
    input: [
      {
        type: "message",
        role: "assistant",
        id: "msg_ci_history",
        phase: "commentary",
        content: [{ type: "output_text", text: "Plain history" }],
      },
      { type: "reasoning", id: "rs_ci", encrypted_content: ciphertext },
      {
        type: "agent_message",
        content: [
          { type: "input_text", text: "Plain handoff" },
          { type: "encrypted_content", encrypted_content: ciphertext },
        ],
      },
      {
        type: "function_call",
        id: "fc_ci",
        call_id: "call_ci",
        name: "synthetic_tool",
        arguments: '{"encrypted_content":"user-data","id":3}',
      },
      {
        type: "function_call_output",
        call_id: "call_ci",
        output: "Tool result",
      },
      { role: "user", content: "Continue the synthetic test" },
    ],
  };
  async function send(body) {
    const response = await fetch("http://127.0.0.1:15731/v1/responses", {
      method: "POST",
      signal: AbortSignal.timeout(20_000),
      headers: {
        "content-type": "application/json",
        authorization: "Bearer fake-client-key",
      },
      body: JSON.stringify(body),
    });
    return {
      status: response.status,
      retryAfter: response.headers.get("retry-after"),
      contentEncoding: response.headers.get("content-encoding"),
      text: await response.text(),
    };
  }
  try {
    await cdp.invoke("add_provider", {
      app: "codex",
      provider,
      addToLive: false,
    });
    await cdp.invoke("switch_provider", { app: "codex", id: provider.id });
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: false,
      },
    });
    await cdp.invoke("start_proxy_server");
    proxyStarted = true;
    try {
      await openCompatibilitySettings(cdp, until);
    } catch (error) {
      await cdp.screenshot(
        `${label.replaceAll(" ", "-")}-settings-failure.png`,
      );
      const controls = await cdp.evaluate(`JSON.stringify({
        tabs: [...document.querySelectorAll('[role="tab"]')].map(item => ({text:item.innerText, selected:item.getAttribute('aria-selected')})),
        sections: [...document.querySelectorAll('button[aria-expanded]')].map(item => ({text:item.innerText, expanded:item.getAttribute('aria-expanded')}))
      })`);
      throw new Error(`${error.message}; settings controls: ${controls}`);
    }
    await setCompatibility(cdp, until, false);
    const disabled = await send(history);
    assert.equal(disabled.status, 400);
    assert.match(disabled.text, /invalid_encrypted_content/);
    assert.equal(hits, 1);
    assert.equal(
      requests.at(-1).authorization,
      "Bearer fake-yunswitch-codex-key",
    );
    assert.equal(requests.at(-1).path, "/v1/responses");
    check(`${label} Codex compatibility defaults off and preserves history`);

    await setCompatibility(cdp, until, true);
    assert.deepEqual(await cdp.invoke("get_rectifier_config"), {
      ...rectifierBefore,
      codexResponsesCompatibility: true,
    });
    for (const stream of [true, false]) {
      const result = await send({ ...history, stream });
      assert.equal(result.status, 200, result.text);
      assert.match(result.text, /YunSwitch Codex smoke/);
      assert.doesNotMatch(result.text, /invalid_encrypted_content/);
      const outgoing = requests.at(-1).body;
      assert.ok(!JSON.stringify(outgoing).includes(ciphertext));
      assert.equal(outgoing.input[0].id, undefined);
      assert.equal(outgoing.input[0].phase, "commentary");
      const call = outgoing.input.find((item) => item.type === "function_call");
      const resultItem = outgoing.input.find(
        (item) => item.type === "function_call_output",
      );
      assert.equal(call.id, undefined);
      assert.equal(call.call_id, resultItem.call_id);
      assert.equal(call.arguments, history.input[3].arguments);
    }
    check(`${label} Codex compatibility repairs JSON and SSE gateway requests`);
    await cdp.screenshot(
      `${label.replaceAll(" ", "-")}-codex-compatibility.png`,
    );

    const hitsBeforeOpaque = hits;
    const opaque = await send({
      model: "ci-model",
      input: [{ type: "compaction", encrypted_content: ciphertext }],
    });
    assert.equal(opaque.status, 400, opaque.text);
    assert.equal(hits, hitsBeforeOpaque);
    assert.match(opaque.text, /Replay complete explicit history/);
    assert.ok(!opaque.text.includes(ciphertext));
    check(`${label} Codex opaque history is rejected before reaching upstream`);

    scenario = "retry-429";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: true,
        rateLimitMaxWaitSecs: 5,
      },
    });
    const retry = await send({
      model: "ci-model",
      input: "Retry",
      stream: false,
    });
    assert.equal(retry.status, 200, retry.text);
    assert.equal(hits, 2);
    const status = await cdp.invoke("get_proxy_status");
    const wireStatus = await fetch("http://127.0.0.1:15731/status", {
      signal: AbortSignal.timeout(5_000),
    }).then((response) => response.json());
    assert.equal(status.running, true);
    assert.equal(wireStatus.running, true);
    assert.deepEqual(
      status.rate_limit_waits_by_app,
      wireStatus.rate_limit_waits_by_app,
    );
    assert.ok(
      status.rate_limit_waits_by_app.codex,
      `Missing Codex wait bucket: ${JSON.stringify({ status, wireStatus })}`,
    );
    assert.equal(status.rate_limit_waits_by_app.codex.count, 1);
    assert.equal(status.rate_limit_waits_by_app.codex.totalSecs, 1);
    check(
      `${label} Codex retries once after Retry-After with failover disabled`,
    );

    scenario = "always-429";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        rateLimitRetryEnabled: false,
      },
    });
    const limited = await send({ model: "ci-model", input: "Rate limit" });
    assert.equal(limited.status, 429, limited.text);
    assert.equal(limited.retryAfter, "1");
    assert.equal(JSON.parse(limited.text).error.type, "rate_limit_error");
    assert.equal(JSON.parse(limited.text).error.retry_after_seconds, 1);
    assert.equal(hits, 1);
    check(`${label} final Codex rate limit preserves HTTP 429 and Retry-After`);

    await setCompatibility(cdp, until, false);
    assert.deepEqual(await cdp.invoke("get_rectifier_config"), rectifierBefore);
    await setCompatibility(cdp, until, true);
    check(
      `${label} compatibility UI saves on and off without changing other settings`,
    );

    // Reuse the fetch connection pool across stop/start. It must reconnect to
    // the new instance instead of serving requests with a stopped status object.
    await cdp.invoke("stop_proxy_server");
    proxyStarted = false;
    await cdp.invoke("start_proxy_server");
    proxyStarted = true;
    scenario = "retry-429";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: true,
        rateLimitMaxWaitSecs: 5,
      },
    });
    const afterRestart = await send({
      model: "ci-model",
      input: "After restart",
    });
    assert.equal(afterRestart.status, 200, afterRestart.text);
    assert.equal(hits, 2);
    const newStatus = await cdp.invoke("get_proxy_status");
    assert.equal(newStatus.rate_limit_waits_by_app.codex.count, 1);
    assert.equal(newStatus.total_requests, 1);
    const newWireStatus = await fetch("http://127.0.0.1:15731/status", {
      signal: AbortSignal.timeout(5_000),
    }).then((response) => response.json());
    assert.equal(newWireStatus.running, true);
    assert.deepEqual(
      newWireStatus.rate_limit_waits_by_app,
      newStatus.rate_limit_waits_by_app,
    );
    check(
      `${label} restarted proxy retires pooled connections and keeps live statistics`,
    );

    scenario = "retry-stream-429";
    hits = 0;
    const earlyLimited = await send({
      model: "ci-model",
      input: "Early limit",
      stream: true,
    });
    assert.equal(earlyLimited.status, 200, earlyLimited.text);
    assert.equal(hits, 2);
    assert.match(earlyLimited.text, /YunSwitch Codex smoke/);
    assert.doesNotMatch(earlyLimited.text, /response.failed/);
    const afterStreamWait = await cdp.invoke("get_proxy_status");
    assert.equal(afterStreamWait.rate_limit_waits_by_app.codex.count, 2);
    check(label + " Codex HTTP 200 stream rate limits retry before output");

    scenario = "retry-stream-503";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 1,
        sameProviderRetryBackoffMs: 1,
        sameProviderRetryMaxBackoffMs: 1,
      },
    });
    const earlyServer = await send({
      model: "ci-model",
      input: "Early server error",
      stream: true,
    });
    assert.equal(earlyServer.status, 200, earlyServer.text);
    assert.equal(hits, 2);
    assert.match(earlyServer.text, /YunSwitch Codex smoke/);
    assert.doesNotMatch(earlyServer.text, /response.failed/);
    check(label + " Codex stream service errors obey same-provider retry");

    scenario = "late-stream-error";
    hits = 0;
    const partial = await send({
      model: "ci-model",
      input: "Partial output",
      stream: true,
    });
    assert.equal(partial.status, 200, partial.text);
    assert.equal(hits, 1);
    assert.equal(partial.text.match(/only-once/g)?.length, 1);
    assert.match(partial.text, /response.failed/);
    check(label + " Codex errors after output never replay text or tools");

    scenario = "always-stream-429";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: false,
      },
    });
    const finalStreamLimit = await send({
      model: "ci-model",
      input: "Final stream limit",
      stream: true,
    });
    assert.equal(finalStreamLimit.status, 429, finalStreamLimit.text);
    assert.equal(finalStreamLimit.retryAfter, "1");
    assert.equal(
      JSON.parse(finalStreamLimit.text).error.type,
      "rate_limit_error",
    );
    assert.equal(hits, 1);
    check(
      label + " Codex final stream rate limit preserves 429 and Retry-After",
    );

    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: true,
        rateLimitMaxWaitSecs: 10,
      },
    });
    for (const coding of ["gzip", "brotli"]) {
      scenario = `retry-${coding}-stream-429`;
      hits = 0;
      const compressed = await send({
        model: "ci-model",
        input: "Compressed early limit",
        stream: true,
      });
      assert.equal(compressed.status, 200, compressed.text);
      assert.equal(compressed.contentEncoding, null);
      assert.match(compressed.text, /YunSwitch Codex smoke/);
      assert.doesNotMatch(compressed.text, /response.failed/);
      assert.equal(hits, 2);
    }
    check(
      label +
        " Codex gzip and Brotli early rate limits recover without failed output",
    );

    scenario = "late-gzip-stream-error";
    hits = 0;
    const compressedPartial = await send({
      model: "ci-model",
      input: "Compressed partial output",
      stream: true,
    });
    assert.equal(compressedPartial.status, 200, compressedPartial.text);
    assert.equal(compressedPartial.contentEncoding, null);
    assert.equal(hits, 1);
    assert.equal(compressedPartial.text.match(/only-once/g)?.length, 1);
    assert.match(compressedPartial.text, /response.failed/);
    check(label + " Codex compressed errors after output never replay");

    scenario = "always-gzip-stream-429";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: false,
      },
    });
    const compressedFinal = await send({
      model: "ci-model",
      input: "Compressed final limit",
      stream: true,
    });
    assert.equal(compressedFinal.status, 429, compressedFinal.text);
    assert.equal(compressedFinal.retryAfter, "1");
    assert.equal(
      JSON.parse(compressedFinal.text).error.type,
      "rate_limit_error",
    );
    assert.equal(hits, 1);
    check(
      label + " Codex final compressed limit preserves 429 and Retry-After",
    );

    scenario = "cancel-wait";
    hits = 0;
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 0,
        rateLimitRetryEnabled: true,
        rateLimitMaxWaitSecs: 10,
      },
    });
    const cancelledBody = JSON.stringify({
      model: "ci-model",
      input: "Cancelled retry",
      stream: true,
    });
    const client = connect({ host: "127.0.0.1", port: 15731 });
    try {
      await new Promise((done, reject) =>
        client.once("connect", done).once("error", reject),
      );
      client.write(
        `POST /v1/responses HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: ${Buffer.byteLength(cancelledBody)}\r\n\r\n${cancelledBody}`,
      );
      await until(
        async () =>
          Boolean((await cdp.invoke("get_proxy_status")).rate_limit_waiting),
        "cancelled request enters wait",
      );
      assert.equal(hits, 1);
      const concurrent = await send({
        model: "ci-model",
        input: "Concurrent success",
      });
      assert.equal(concurrent.status, 200, concurrent.text);
      assert.ok(
        (await cdp.invoke("get_proxy_status")).rate_limit_waiting,
        "another request cleared the wait",
      );
      client.destroy();
      await until(async () => {
        const state = await cdp.invoke("get_proxy_status");
        return state.active_connections === 0 && !state.rate_limit_waiting;
      }, "cancelled request cleans waiting and connection state");
      assert.equal((await cdp.invoke("get_proxy_status")).last_error, null);
      await new Promise((done) => setTimeout(done, 5200));
      assert.equal(hits, 2, "cancelled request retried after its wait");
      check(
        label +
          " Codex concurrent success preserves waiting and cancellation stops retries",
      );
    } finally {
      client.destroy();
    }

    await setCompatibility(cdp, until, false);
    await cdp.invoke("update_proxy_config_for_app", {
      config: {
        ...proxyBefore,
        autoFailoverEnabled: false,
        sameProviderRetries: 2,
        sameProviderRetryBackoffMs: 1,
        sameProviderRetryMaxBackoffMs: 1,
        rateLimitRetryEnabled: true,
        rateLimitMaxWaitSecs: 5,
      },
    });
    scenario = "stateful-503";
    for (const extra of [
      { background: true },
      { previous_response_id: "resp_ci_existing" },
      { conversation: "conv_ci_existing" },
      { tools: [{ type: "web_search" }] },
    ]) {
      hits = 0;
      const outcome = await send({
        model: "ci-model",
        input: "Run once",
        ...extra,
      });
      assert.equal(outcome.status, 503, outcome.text);
      assert.equal(hits, 1, "stateful HTTP error was replayed");
    }
    check(label + " Codex stateful HTTP errors are never replayed");

    scenario = "always-429";
    hits = 0;
    const waitsBeforeStateful = (await cdp.invoke("get_proxy_status"))
      .rate_limit_wait_count;
    const limitedState = await send({
      model: "ci-model",
      input: "Run once",
      background: true,
    });
    assert.equal(limitedState.status, 429, limitedState.text);
    assert.equal(limitedState.retryAfter, "1");
    assert.equal(hits, 1);
    assert.equal(
      (await cdp.invoke("get_proxy_status")).rate_limit_wait_count,
      waitsBeforeStateful,
    );
    check(
      label + " Codex stateful rate limits return Retry-After without replay",
    );

    for (const [name, seconds] of [
      ["body-wait-hint", "17"],
      ["repeated-wait-hint", "23"],
    ]) {
      scenario = name;
      hits = 0;
      const hinted = await send({ model: "ci-model", input: "Wait guidance" });
      assert.equal(hinted.status, 429, hinted.text);
      assert.equal(hinted.retryAfter, seconds);
      assert.equal(hits, 1, "a wait longer than the cap must not retry");
    }
    check(
      label +
        " Codex body and repeated header wait hints preserve the longer delay",
    );

    scenario = "cooldown-race";
    hits = 0;
    releaseEarlierSuccess = undefined;
    const earlier = send({ model: "ci-model", input: "Earlier success" });
    try {
      await until(
        () => Boolean(releaseEarlierSuccess),
        "earlier request reaches upstream",
      );
      const newer = await send({
        model: "ci-model",
        input: "Newer rate limit",
      });
      assert.equal(newer.status, 429, newer.text);
      releaseEarlierSuccess();
      const success = await earlier;
      assert.equal(success.status, 200, success.text);
      const cooling = await cdp.invoke("get_rate_limit_cooldowns", {
        appType: "codex",
      });
      assert.ok(
        cooling.some(
          (item) => item.providerId === provider.id && item.remainingSecs > 0,
        ),
        "old success erased a newer cooldown",
      );
      assert.equal(hits, 2);
      check(
        label + " Codex older successes preserve newer concurrent cooldowns",
      );
    } finally {
      releaseEarlierSuccess?.();
      await earlier.catch(() => {});
    }

    scenario = "quota-429";
    hits = 0;
    const waitsBeforeQuota = (await cdp.invoke("get_proxy_status"))
      .rate_limit_wait_count;
    const exhausted = await send({
      model: "ci-model",
      input: "Quota exhaustion",
    });
    assert.equal(hits, 1);
    assert.notEqual(exhausted.status, 429, exhausted.text);
    assert.equal(
      JSON.parse(exhausted.text).error.code,
      "cc_switch_billing_exhausted",
    );
    assert.equal(
      (await cdp.invoke("get_proxy_status")).rate_limit_wait_count,
      waitsBeforeQuota,
    );
    const exhaustedHealth = await cdp.invoke("get_provider_health", {
      providerId: provider.id,
      appType: "codex",
    });
    assert.equal(exhaustedHealth.fatal, true);
    check(
      label +
        " Codex HTTP quota exhaustion never enters the rate limit retry loop",
    );
    // Preserve the established relaunch acceptance preference.
    await setCompatibility(cdp, until, true);
  } finally {
    try {
      if (proxyStarted) await cdp.invoke("stop_proxy_with_restore");
      await cdp.invoke("update_proxy_config_for_app", { config: proxyBefore });
    } finally {
      upstream.closeAllConnections();
      await new Promise((done) => upstream.close(done));
    }
  }
}

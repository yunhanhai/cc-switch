// Called only after windows-release-smoke.mjs validates a disposable runner.
import assert from "node:assert/strict";
import { createServer } from "node:http";

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
        tab.click();
        return true;
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
      if (
        scenario === "always-429" ||
        (scenario === "retry-429" && hits === 1)
      ) {
        response.writeHead(429, {
          "content-type": "application/json",
          "retry-after": "1",
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
      response.writeHead(200, {
        "content-type": body.stream ? "text/event-stream" : "application/json",
      });
      response.end(
        body.stream
          ? `event: response.completed\ndata: ${JSON.stringify({ type: "response.completed", response: result })}\n\n`
          : JSON.stringify(result),
      );
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
    await openCompatibilitySettings(cdp, until);
    await setCompatibility(cdp, until, false);
    const disabled = await send(history);
    assert.equal(disabled.status, 200);
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

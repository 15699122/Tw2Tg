import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { acceptedRecordSelection } from "../src/lib/record-import.js";
import {
  configurationExchangeApplyMessage,
  configurationExchangeCancelMessage,
} from "../src/lib/ui-state.js";

test("mixed preview verdicts preserve original record indices", () => {
  const entries = ["preserve", "accept", "preserve", "accept"]
    .map((verdict) => ({ verdict }));
  assert.deepEqual(acceptedRecordSelection({ entries }), [{ index: 1 }, { index: 3 }]);
  assert.deepEqual(acceptedRecordSelection({ entries }, [0, 3]), [{ index: 3 }]);
  assert.deepEqual(acceptedRecordSelection({ entries }, []), []);
  assert.deepEqual(acceptedRecordSelection({ entries: [] }), []);
  assert.deepEqual(acceptedRecordSelection({ entries: [{ verdict: "preserve" }] }), []);
});

test("both input setters invalidate preview and acceptance state", () => {
  const source = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
  for (const setter of ["setRecordsJson", "setArchiveRoot"]) {
    assert.match(source, new RegExp(`${setter}=\\{\\(value\\) => \\{ ${setter}\\(value\\); setPreviewResult\\(null\\); setAcceptResult\\(null\\);`));
  }
});

test("Tauri invocations use camelCase arguments and acceptance carries preview revision", () => {
  const source = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
  assert.match(source, /invoke\("preview_record_import", \{ recordsJson, archiveRoot \}\)/);
  assert.match(source, /expectedRevision: previewResult\.revision/);
  assert.doesNotMatch(source, /records_json: recordsJson/);
});

test("record exchange page exposes canonical exports and cancellable acceptance", () => {
  const source = readFileSync(new URL("../src/pages/preview-accept-page.jsx", import.meta.url), "utf8");
  assert.match(source, /invoke\("export_record_exchange", \{ jobId: exportJobId, format, kind: exportKind, archiveRoot, tweetId: exportTweetId, batchId: exportBatchId \}\)/);
  assert.match(source, /invoke\("cancel_record_import"\)/);
  assert.match(source, /\$\{exportKind\}-records\.\$\{format\}/);
});
test("configuration exchange apply wait is cancellable end to end", () => {
  const page = readFileSync(new URL("../src/pages/download-config-page.jsx", import.meta.url), "utf8");
  const main = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
  const backend = readFileSync(new URL("../src-tauri/src/commands/records.rs", import.meta.url), "utf8");
  const config = readFileSync(new URL("../src-tauri/src/config.rs", import.meta.url), "utf8");
  const lib = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
  // This test is wiring only. The behavioural proof for cancellation lives in
  // the Rust tests `cancel_command_reaches_a_blocked_maintenance_drain` and
  // `apply_flow_cancelled_after_the_wait_writes_nothing_and_still_applies_later`;
  // no assertion here may be described as end-to-end evidence.
  assert.match(backend, /pub\(crate\) fn cancel_configuration_exchange\(\)/);
  assert.match(backend, /cancellation\.is_cancelled\(\) \|\| std::time::Instant::now\(\) >= deadline/);
  // A blocking drain must not run inline on the IPC thread, otherwise the
  // cancel command can never be delivered while apply waits.
  assert.match(backend, /pub\(crate\) async fn apply_configuration_exchange\(/);
  assert.match(backend, /tauri::async_runtime::spawn_blocking\(/);
  // A cancelled wait must be classified as cancellation, not as a lock failure.
  assert.match(backend, /configuration exchange apply cancelled while waiting/);
  // The cancel-vs-write window has an explicit acceptance point right before
  // the temporary file is created and renamed.
  assert.match(config, /configuration exchange apply cancelled before writing/);
  assert.match(config, /cancellation\.is_some_and\(\|token\| token\.is_cancelled\(\)\)/);
  assert.match(lib, /commands::records::cancel_configuration_exchange/);
  // The GUI wires the command so a user can actually interrupt the wait.
  assert.match(main, /invoke\("cancel_configuration_exchange"\)/);
  assert.match(page, /onClick=\{cancelExchange\}/);
  assert.match(main, /cancelExchange=\{cancelExchange\}/);
});

test("apply and cancel messages cannot overwrite a settled result", () => {
  // A cancelled wait is not a lock failure and not a plain apply failure.
  assert.match(configurationExchangeApplyMessage("configuration exchange apply cancelled while waiting for workspace maintenance"), /已取消等待；本机配置未改动/);
  // A stale revision must never be reported as a cancellation.
  assert.doesNotMatch(configurationExchangeApplyMessage("configuration preview is stale; preview again"), /已取消/);
  assert.match(configurationExchangeApplyMessage("configuration preview is stale; preview again"), /重新预览/);
  // A real maintenance failure stays a failure.
  assert.match(configurationExchangeApplyMessage("workspace maintenance unavailable: archive coordination lock is held by another owner"), /应用失败/);
  // A genuine success reply never reaches this helper, so anything else fails.
  assert.match(configurationExchangeApplyMessage("unexpected"), /应用失败：unexpected/);

  // A cancel acknowledgement only speaks while its own apply is still pending.
  assert.match(
    configurationExchangeCancelMessage({ attempt: 2, currentAttempt: 2, settled: false }),
    /已请求取消/,
  );
  assert.equal(
    configurationExchangeCancelMessage({ attempt: 2, currentAttempt: 2, settled: true }),
    "",
    "a settled apply must not be overwritten by a late cancel acknowledgement",
  );
  assert.equal(
    configurationExchangeCancelMessage({ attempt: 1, currentAttempt: 2, settled: false }),
    "",
    "a stale attempt must not write into a newer one",
  );
});

test("main.jsx guards apply and cancel results by attempt", () => {
  const source = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
  // The guard has to exist in the real handler, not only in the helper.
  assert.match(source, /exchangeAttempt\.current !== attempt/);
  assert.match(source, /configurationExchangeApplyMessage\(reason\)/);
  assert.match(source, /configurationExchangeCancelMessage\(\{ attempt, currentAttempt: exchangeAttempt\.current, settled: exchangeSettled\.current \}\)/);
});

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { telegramActions, telegramSettingsInput } from "../src/lib/telegram-state.js";

test("settings input contains only writable non-secret fields", () => {
  const input = telegramSettingsInput({ enabled: true, chat_id: "123", token: "must-not-pass", bot_token_present: true, revision: 90, capability: {} });
  assert.equal(input.enabled, true);
  for (const key of ["token", "bot_token_present", "revision", "capability", "migration_pending"]) assert.equal(key in input, false);
});
test("runtime-owned migration and capability flags cannot be set by the settings form", () => {
  const input = telegramSettingsInput({ enabled: false, migration_pending: false, capability: { cloud: true } });
  assert.equal(input.enabled, false);
  for (const key of ["migration_pending", "capability"]) assert.equal(key in input, false);
});
test("unknown, confirmed and inflight outcomes cannot be automatically retried or cleanly cancelled", () => {
  assert.deepEqual(telegramActions("UNKNOWN"), { cancel: false, review: true });
  for (const state of ["SENT", "IN_FLIGHT", "FAILED_PERMANENT", "CANCELLED"]) assert.equal(telegramActions(state).cancel, false);
  for (const state of ["QUEUED", "RETRY_WAIT"]) assert.equal(telegramActions(state).cancel, true);
});
test("settings and task surfaces wire registered commands without persistent token storage", () => {
  const settings = readFileSync(new URL("../src/components/telegram-settings.jsx", import.meta.url), "utf8");
  const jobs = readFileSync(new URL("../src/components/telegram-job-state.jsx", import.meta.url), "utf8");
  const backend = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
  for (const command of ["get_telegram_settings", "save_telegram_settings", "migrate_telegram_endpoint", "replace_telegram_credential", "delete_telegram_credential", "confirm_telegram_resume", "start_telegram_sender", "stop_telegram_sender", "inspect_telegram_connection"]) {
    assert.ok(settings.includes(`"${command}"`), `settings: ${command}`);
    assert.ok(backend.includes(`commands::${command}`), `backend: ${command}`);
  }
  // Cancellation belongs to the task surface, not the settings surface.
  for (const command of ["get_telegram_job_state", "cancel_telegram_send", "review_telegram_unknown"]) {
    assert.ok(jobs.includes(`"${command}"`), `jobs: ${command}`);
    assert.ok(backend.includes(`commands::${command}`), `backend: ${command}`);
  }
  assert.match(settings, /type="password"/);
  assert.match(settings, /setToken\(""\)/);
  assert.doesNotMatch(settings, /localStorage|sessionStorage|console\.|emitFrontendEvent/);
  assert.match(jobs, /get_telegram_job_state/);
  assert.match(jobs, /clearInterval/);
});

test("endpoint migration requires explicit confirmation and surfaces the paused state", () => {
  const settings = readFileSync(new URL("../src/components/telegram-settings.jsx", import.meta.url), "utf8");
  assert.match(settings, /migrate_telegram_endpoint/);
  assert.match(settings, /confirmed: true/);
  assert.match(settings, /window\.confirm/);
  assert.match(settings, /migration_pending/);
  // The pause is owned by the backend; the form must not offer a silent switch.
  assert.doesNotMatch(settings, /onClick=\{\(\) => action\("save_telegram_settings"[^}]*endpoint_mode/);
});
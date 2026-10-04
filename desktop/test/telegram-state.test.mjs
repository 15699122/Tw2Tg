import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { telegramActions, telegramSettingsInput } from "../src/lib/telegram-state.js";

test("settings input contains only writable non-secret fields", () => {
  const input = telegramSettingsInput({ enabled: true, chat_id: "123", token: "must-not-pass", bot_token_present: true, revision: 90, capability: {} });
  assert.equal(input.enabled, true);
  for (const key of ["token", "bot_token_present", "revision", "capability"]) assert.equal(key in input, false);
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
  for (const command of ["get_telegram_settings", "save_telegram_settings", "replace_telegram_credential", "delete_telegram_credential", "confirm_telegram_resume", "start_telegram_sender", "stop_telegram_sender"]) {
    assert.ok(settings.includes(`"${command}"`)); assert.ok(backend.includes(`commands::${command}`));
  }
  assert.match(settings, /type="password"/);
  assert.match(settings, /setToken\(""\)/);
  assert.doesNotMatch(settings, /localStorage|sessionStorage|console\.|emitFrontendEvent/);
  assert.match(jobs, /get_telegram_job_state/);
  assert.match(jobs, /clearInterval/);
});
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("Telegram settings and job projection are mounted separately from archive state", () => {
  const settings = readFileSync(new URL("../src/pages/settings-page.jsx", import.meta.url), "utf8");
  const shared = readFileSync(new URL("../src/pages/shared.jsx", import.meta.url), "utf8");
  assert.match(settings, /<TelegramSettings expanded=\{expandedSections\.telegram \?\? false\}/);
  // TelegramJobState was retired from the shared job row when task history moved
  // to the paginated Downloads page; the component file is preserved for the
  // Windows owner's TG-06 re-integration and must not be mounted from shared.jsx.
  assert.doesNotMatch(shared, /TelegramJobState/);
  assert.doesNotMatch(shared, /telegram-job-state/);
  const task = readFileSync(new URL("../src/components/telegram-job-state.jsx", import.meta.url), "utf8");
  assert.doesNotMatch(task, /retry_telegram|已读|已接收/);
  assert.match(task, /rel="noreferrer"/);
  assert.match(task, /row\.progress/);
});
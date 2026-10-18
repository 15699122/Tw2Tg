import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { acceptedRecordSelection } from "../src/lib/record-import.js";

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
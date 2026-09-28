import assert from "node:assert/strict";
import fs from "node:fs";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, it } from "node:test";

import {
  WINDOW_TARGET_STATES,
  captureReadinessFailure,
  collectCurrentDocumentEvidence,
  collectWindowEvidence,
  discoverApplicationWindow,
  ensureArtifactDir,
  isBlankDocument,
  isXArchiveDocument,
  looksLikeApplicationDocument,
  resolveDiagnosticsDir,
  snapshotSessionStart,
  waitForApplicationDocument,
  writeArtifact,
} from "../e2e/support/native-startup.mjs";

/** Absolute, drive-agnostic artifact root (the 2026-09-28 WQ-ENG-13 lesson). */
function tempRoot() {
  return fs.mkdtempSync(path.join(tmpdir(), "xarchive-native-startup-"));
}

function cleanup(root) {
  try {
    rmSync(root, { recursive: true, force: true });
  } catch {
    // best effort
  }
}

/**
 * Minimal DOM stand-in so the module's real `browser.execute` callbacks run
 * against real `document` lookups instead of being mocked away.
 */
async function withDocument(state, fn) {
  const previous = globalThis.document;
  const element = (id) => {
    if (id === "root") {
      return state.rootExists ? { innerText: state.rootText ?? "" } : null;
    }
    if (id === "startup-fallback") {
      return state.startupFallback ? { innerText: state.startupFallback } : null;
    }
    return null;
  };
  globalThis.document = {
    readyState: state.readyState ?? "complete",
    documentElement: { dataset: { xarchiveStartup: state.startupState ?? "" } },
    body: { innerText: state.bodyText ?? "" },
    getElementById: element,
  };
  try {
    return await fn();
  } finally {
    globalThis.document = previous;
  }
}

/**
 * Fake WDIO session. `documents` maps a handle to its DOM state, and
 * `onSample` (if given) can flip the state between discovery attempts.
 */
function fakeBrowser({ handles = ["w-1"], documents = {}, switchFails = [], onSample } = {}) {
  let current = handles[0] ?? null;
  const calls = { switchToWindow: [], execute: 0 };
  return {
    calls,
    capabilities: { browserName: "tauri" },
    getWindowHandles: async () => {
      if (onSample) {
        await onSample(calls);
      }
      return handles;
    },
    getWindowHandle: async () => current,
    switchToWindow: async (handle) => {
      if (switchFails.includes(handle)) {
        throw new Error("invalid window handle");
      }
      current = handle;
      calls.switchToWindow.push(handle);
    },
    getUrl: async () => documents[current]?.url ?? "data:,",
    getTitle: async () => documents[current]?.title ?? "",
    getPageSource: async () => "<html><div id=\"root\"></div></html>",
    saveScreenshot: async (file) => {
      fs.writeFileSync(file, "png");
    },
    execute: async (fn) => {
      calls.execute += 1;
      return fn();
    },
  };
}

describe("blank and application document classification", () => {
  it("recognizes blank document URLs", () => {
    for (const url of ["data:,", "about:blank", "", null, undefined, "  data:,  "]) {
      assert.equal(isBlankDocument(url), true, `${String(url)} should be blank`);
    }
  });

  it("rejects non-blank URLs", () => {
    const urls = ["file:///xarchive", "https://example.com", "tauri://localhost", "http://tauri.localhost/"];
    for (const url of urls) {
      assert.equal(isBlankDocument(url), false, `${url} should not be blank`);
    }
  });

  it("detects XArchive markers and rejects foreign documents", () => {
    assert.equal(isXArchiveDocument({ rootExists: true }), true);
    assert.equal(isXArchiveDocument({ rootExists: false, startupState: "react_mount_completed" }), true);
    assert.equal(isXArchiveDocument({ rootExists: false, startupFallback: "启动失败" }), true);
    assert.equal(isXArchiveDocument({ rootExists: false, title: "XArchive" }), true);
    assert.equal(isXArchiveDocument({ rootExists: false, title: "Other" }), false);
    assert.equal(isXArchiveDocument({ rootExists: true, error: "switch failed" }), false);
    assert.equal(isXArchiveDocument(null), false);
  });

  it("does not mistake an unreadable DOM for a loaded application document", () => {
    // `collectWindowEvidence` records failed reads as `<unavailable: …>` strings.
    const unavailable = "<unavailable: no such window>";
    assert.equal(isXArchiveDocument({ rootExists: unavailable }), false);
    assert.equal(isXArchiveDocument({ rootExists: false, startupState: unavailable }), false);
    assert.equal(
      looksLikeApplicationDocument({ url: "data:,", rootExists: unavailable, title: unavailable }),
      false,
    );
  });

  it("treats a non-blank URL or an XArchive marker as an application document", () => {
    assert.equal(looksLikeApplicationDocument({ url: "http://tauri.localhost/" }), true);
    assert.equal(looksLikeApplicationDocument({ url: "data:,", rootExists: true }), true);
    assert.equal(looksLikeApplicationDocument({ url: "data:,", rootExists: false }), false);
    assert.equal(
      looksLikeApplicationDocument({ url: "data:,", rootExists: false, error: "switch failed" }),
      false,
    );
  });
});

describe("artifact directory contract", () => {
  it("mirrors WDIO_LOG_DIR and keeps READINESS_DIAGNOSTICS as a fallback", () => {
    const root = tempRoot();
    try {
      const defaultDir = path.join(root, "test-artifacts", "wdio", "startup");
      assert.equal(resolveDiagnosticsDir({}, root), defaultDir);
      assert.equal(
        resolveDiagnosticsDir({ WDIO_LOG_DIR: "custom-logs" }, root),
        path.join(root, "custom-logs", "startup"),
      );
      assert.equal(
        resolveDiagnosticsDir({ READINESS_DIAGNOSTICS: "readiness-dir" }, root),
        path.join(root, "readiness-dir", "startup"),
      );
      // WDIO_LOG_DIR is the runner's own log directory, so it wins.
      assert.equal(
        resolveDiagnosticsDir({ WDIO_LOG_DIR: "logs", READINESS_DIAGNOSTICS: "other" }, root),
        path.join(root, "logs", "startup"),
      );
    } finally {
      cleanup(root);
    }
  });

  it("writes JSON and text artifacts inside the startup directory", () => {
    const root = tempRoot();
    try {
      const jsonPath = writeArtifact("evidence.json", { url: "data:,", rootExists: false }, {}, root);
      assert.equal(jsonPath, path.join(root, "test-artifacts", "wdio", "startup", "evidence.json"));
      assert.deepEqual(JSON.parse(readFileSync(jsonPath, "utf8")), {
        url: "data:,",
        rootExists: false,
      });
      const textPath = writeArtifact("trace.log", "hello", {}, root);
      assert.equal(readFileSync(textPath, "utf8"), "hello");
      assert.equal(ensureArtifactDir({}, root), path.dirname(jsonPath));
    } finally {
      cleanup(root);
    }
  });
});

describe("session evidence collection", () => {
  it("records window evidence and restores the previously selected handle", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({
      handles: ["w-1", "w-2"],
      documents: {
        "w-1": { url: "data:,", rootExists: false },
        "w-2": { url: "http://tauri.localhost/", title: "XArchive" },
      },
    });
    try {
      const evidence = await withDocument(
        { rootExists: true, rootText: "工作台", bodyText: "body" },
        () => collectWindowEvidence("w-2", true, browser),
      );
      assert.equal(evidence.handle, "w-2");
      assert.equal(evidence.current, true);
      assert.equal(evidence.switched, true);
      assert.equal(evidence.url, "http://tauri.localhost/");
      assert.equal(evidence.title, "XArchive");
      assert.equal(evidence.readyState, "complete");
      assert.equal(evidence.rootExists, true);
      assert.equal(evidence.error, null);
      // The original handle is selected again before returning.
      assert.deepEqual(browser.calls.switchToWindow, ["w-2", "w-1"]);
    } finally {
      cleanup(root);
    }
  });

  it("reports a failed target switch instead of throwing", async () => {
    const browser = fakeBrowser({ handles: ["w-1", "w-2"], switchFails: ["w-2"] });
    const evidence = await collectWindowEvidence("w-2", false, browser);
    assert.equal(evidence.switched, false);
    assert.match(evidence.error, /switchToWindow: invalid window handle/);
    assert.equal(evidence.url, undefined);
  });

  it("degrades unreadable session commands into evidence fields", async () => {
    const browser = fakeBrowser();
    browser.getUrl = async () => {
      throw new Error("no such window");
    };
    const evidence = await withDocument({ rootExists: false }, () =>
      collectCurrentDocumentEvidence(browser),
    );
    assert.match(evidence.url, /<unavailable: no such window>/);
    assert.equal(evidence.rootExists, false);
    assert.equal(evidence.readyState, "complete");
  });
});

describe("application document discovery", () => {
  it("finds the application document once the target stops being blank", async () => {
    const root = tempRoot();
    const documents = { "w-1": { url: "data:," } };
    let samples = 0;
    const browser = fakeBrowser({
      handles: ["w-1"],
      documents,
      onSample: async () => {
        samples += 1;
        if (samples >= 2) {
          documents["w-1"] = { url: "http://tauri.localhost/", title: "XArchive" };
        }
      },
    });
    try {
      const result = await withDocument({ rootExists: false }, () =>
        waitForApplicationDocument({
          browser,
          maxPollMs: 5000,
          pollMs: 5,
          sampleMs: 5,
          env: { WDIO_LOG_DIR: "logs" },
          root,
        }),
      );
      assert.equal(result.state, WINDOW_TARGET_STATES.APPLICATION_DOCUMENT_FOUND);
      assert.equal(result.selectedHandle, "w-1");
      assert.equal(result.evidence.url, "http://tauri.localhost/");
      const discovery = JSON.parse(
        readFileSync(path.join(root, "logs", "startup", "discovery.json"), "utf8"),
      );
      assert.equal(discovery.finalResult.state, WINDOW_TARGET_STATES.APPLICATION_DOCUMENT_FOUND);
      assert.ok(discovery.attempts.length >= 2);
    } finally {
      cleanup(root);
    }
  });

  it("reports a blank-only session as ONLY_BLANK_DOCUMENTS", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({ handles: ["w-1"], documents: { "w-1": { url: "data:," } } });
    try {
      const snapshot = await withDocument({ rootExists: false }, () =>
        discoverApplicationWindow({
          browser,
          maxPollMs: 60,
          pollMs: 5,
          sampleMs: 5,
          env: {},
          root,
        }),
      );
      assert.equal(snapshot.finalResult.state, WINDOW_TARGET_STATES.ONLY_BLANK_DOCUMENTS);
      assert.equal(snapshot.finalResult.selectedHandle, null);
      assert.equal(snapshot.attempts.at(-1).onlyBlankDocuments, true);
    } finally {
      cleanup(root);
    }
  });

  it("distinguishes a session with no window handles at all", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({ handles: [] });
    try {
      const snapshot = await discoverApplicationWindow({
        browser,
        maxPollMs: 40,
        pollMs: 5,
        sampleMs: 5,
        env: {},
        root,
      });
      assert.equal(snapshot.finalResult.state, WINDOW_TARGET_STATES.NO_WINDOW_HANDLES);
    } finally {
      cleanup(root);
    }
  });

  it("stops with WINDOW_TARGET_SWITCH_FAILED when a handle cannot be selected", async () => {
    const root = tempRoot();
    // The first handle is already selected; the second one cannot be switched to.
    const browser = fakeBrowser({ handles: ["w-1", "w-2"], switchFails: ["w-2"] });
    try {
      const snapshot = await discoverApplicationWindow({
        browser,
        maxPollMs: 2000,
        pollMs: 5,
        sampleMs: 5,
        env: {},
        root,
      });
      assert.equal(snapshot.finalResult.state, WINDOW_TARGET_STATES.WINDOW_TARGET_SWITCH_FAILED);
      assert.equal(snapshot.finalResult.failedHandle, "w-2");
    } finally {
      cleanup(root);
    }
  });

  it("throws a descriptive readiness error with the discovery timeline", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({ handles: ["w-1"], documents: { "w-1": { url: "data:," } } });
    try {
      await assert.rejects(
        withDocument({ rootExists: false }, () =>
          waitForApplicationDocument({
            browser,
            maxPollMs: 60,
            pollMs: 5,
            sampleMs: 5,
            env: {},
            root,
          }),
        ),
        (error) => {
          assert.equal(error.name, "WaitForApplicationDocumentError");
          assert.equal(error.state, WINDOW_TARGET_STATES.ONLY_BLANK_DOCUMENTS);
          assert.match(error.message, /blank document/);
          assert.match(error.message, /discovery\.json/);
          assert.equal(error.timeline.finalResult.state, WINDOW_TARGET_STATES.ONLY_BLANK_DOCUMENTS);
          return true;
        },
      );
    } finally {
      cleanup(root);
    }
  });
});

describe("failure capture and session-start snapshot", () => {
  it("writes failure.json, the page source and a screenshot without throwing", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({ handles: ["w-1"], documents: { "w-1": { url: "data:," } } });
    try {
      const failure = await withDocument({ rootExists: false }, () =>
        captureReadinessFailure("dashboard.readiness", new Error("readiness timeout"), {
          browser,
          env: {},
          root,
          extraEvidence: { phase: "application-document" },
        }),
      );
      const dir = path.join(root, "test-artifacts", "wdio", "startup");
      assert.equal(failure.label, "dashboard.readiness");
      assert.equal(failure.message, "readiness timeout");
      assert.equal(failure.url, "data:,");
      assert.equal(failure.screenshotPath, path.join(dir, "dashboard-startup-failure.png"));
      assert.equal(fs.existsSync(failure.screenshotPath), true);
      assert.equal(fs.existsSync(path.join(dir, "current-page.html")), true);
      const written = JSON.parse(readFileSync(path.join(dir, "failure.json"), "utf8"));
      assert.deepEqual(written.extraEvidence, { phase: "application-document" });
    } finally {
      cleanup(root);
    }
  });

  it("still records the failure when screenshot and page source are unavailable", async () => {
    const root = tempRoot();
    const browser = fakeBrowser();
    browser.saveScreenshot = async () => {
      throw new Error("no such session");
    };
    browser.getPageSource = async () => {
      throw new Error("no such session");
    };
    try {
      const failure = await withDocument({ rootExists: false }, () =>
        captureReadinessFailure("dashboard.readiness", new Error("boom"), {
          browser,
          env: {},
          root,
        }),
      );
      const dir = path.join(root, "test-artifacts", "wdio", "startup");
      assert.match(failure.screenshotError, /no such session/);
      assert.equal(fs.existsSync(path.join(dir, "failure.json")), true);
      assert.equal(fs.existsSync(path.join(dir, "current-page-error.json")), true);
    } finally {
      cleanup(root);
    }
  });

  it("records the session-start artifact and keeps it after session commands fail", async () => {
    const root = tempRoot();
    const browser = fakeBrowser({ handles: ["w-1", "w-2"], documents: { "w-1": { url: "data:," } } });
    try {
      const snapshot = await snapshotSessionStart("unit-test", browser, { env: {}, root });
      assert.equal(snapshot.kind, "session-start");
      assert.equal(snapshot.label, "unit-test");
      assert.deepEqual(snapshot.windowHandles, ["w-1", "w-2"]);
      assert.equal(snapshot.currentUrl, "data:,");
      assert.equal(snapshot.capabilities.browserName, "tauri");
      assert.equal(snapshot.error, null);
      const written = JSON.parse(
        readFileSync(path.join(root, "test-artifacts", "wdio", "startup", "session-start.json"), "utf8"),
      );
      assert.equal(written.label, "unit-test");
      assert.equal(written.windowHandles.length, 2);

      const broken = fakeBrowser();
      broken.getWindowHandles = async () => {
        throw new Error("invalid session id");
      };
      const failed = await snapshotSessionStart("unit-test-error", broken, { env: {}, root });
      assert.match(failed.error, /invalid session id/);
      assert.deepEqual(failed.windowHandles, []);
      assert.equal(failed.currentHandle, null);
    } finally {
      cleanup(root);
    }
  });
});

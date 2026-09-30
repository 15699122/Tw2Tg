import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, parse, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import {
  componentPlan,
  createManifest,
  filterPackageFiles,
  packageDirectories,
  portablePackageReleaseTag,
  portablePackageVersion,
  validatePackageType,
  validatePortableOutputDir,
} from "../scripts/portable-package.mjs";
import { spawnOptionsForCommand } from "../scripts/build-portable-windows.mjs";

test("portable package type accepts only full and core", () => {
  assert.equal(validatePackageType("full"), "full");
  assert.equal(validatePackageType("core"), "core");
  assert.throws(() => validatePackageType("lite"), /must be full or core/);
});

test("Full package includes gallery-dl directory while Core does not", () => {
  assert.ok(packageDirectories("full").includes("sidecar/gallery-dl"));
  assert.ok(!packageDirectories("core").includes("sidecar/gallery-dl"));
  assert.ok(packageDirectories("core").includes("sidecar/xarchive-downloader"));
  assert.ok(packageDirectories("full").includes("native-host"));
  assert.ok(!packageDirectories("core").includes("native-host"));
});

test("component plan requires worker, keeps aria2 optional, bundles gallery-dl only in Full", () => {
  const full = componentPlan("/project", "/output", "full");
  const core = componentPlan("/project", "/output", "core");
  assert.equal(full.find(([source]) => source.endsWith("xarchive-downloader"))[2], "required");
  assert.equal(full.find(([source]) => source.endsWith("aria2"))[2], "optional");
  assert.equal(full.find(([source]) => source.endsWith("gallery-dl"))[2], "required");
  assert.equal(core.find(([source]) => source.endsWith("xarchive-downloader"))[2], "required");
  assert.equal(core.find(([source]) => source.endsWith("aria2"))[2], "optional");
  assert.equal(core.find(([source]) => source.endsWith("gallery-dl"))[2], "excluded");
});

test("component plan excludes gallery-dl from Core even if source directory exists", () => {
  const core = componentPlan("/project", "/output", "core");
  const entry = core.find(([source]) => source.endsWith("gallery-dl"));
  assert.equal(entry[2], "excluded");
  // Build script must skip excluded components regardless of source existence.
  const galleryDlSuffix = join("sidecar", "gallery-dl");
  assert.equal(entry[0].endsWith(galleryDlSuffix), true);
  assert.equal(entry[1].endsWith(galleryDlSuffix), true);
});

test("manifest explicitly describes Full/Core component boundaries", () => {
  const full = createManifest("full");
  const core = createManifest("core");
  assert.equal(full.sidecar.gallery_dl, "sidecar/gallery-dl/gallery-dl.exe");
  assert.equal(full.extension.bundled, true);
  assert.equal(core.sidecar.gallery_dl, null);
  assert.equal(core.sidecar.gallery_dl_bundled, false);
  assert.equal(core.extension.bundled, false);
  assert.equal(core.extension.user_importable, false);
  assert.equal(full.native_host, null);
});

// ENG-01: the packaging script removes the output directory recursively, so a
// misconfigured PORTABLE_OUTPUT_DIR must fail before any delete happens.
const projectRoot = resolve(sep, "workspace", "Tw2Tg");
const deepProjectRoot = resolve(sep, "data", "builds", "agent", "Tw2Tg");
const homeDir = resolve(sep, "home", "operator");

test("portable output accepts the default dist-portable namespace", () => {
  assert.equal(
    validatePortableOutputDir("dist-portable/XArchive", { projectRoot, homeDir }),
    resolve(projectRoot, "dist-portable", "XArchive"),
  );
  assert.equal(
    validatePortableOutputDir(join("dist-portable", "nested", "pkg"), { projectRoot, homeDir }),
    resolve(projectRoot, "dist-portable", "nested", "pkg"),
  );
});

test("portable output refuses the project root and its ancestors", () => {
  assert.throws(() => validatePortableOutputDir(".", { projectRoot, homeDir }), /project root/);
  assert.throws(
    () => validatePortableOutputDir(projectRoot, { projectRoot, homeDir }),
    /project root/,
  );
  // `deepProjectRoot` has four segments, so `..` stays a real ancestor
  // directory instead of collapsing onto the filesystem root.
  assert.throws(
    () => validatePortableOutputDir(join("..", ".."), { projectRoot: deepProjectRoot, homeDir }),
    /ancestor of the project root/,
  );
  assert.throws(
    () => validatePortableOutputDir(join("..", "..", ".."), { projectRoot: deepProjectRoot, homeDir }),
    /ancestor of the project root/,
  );
});

test("portable output refuses a filesystem root", () => {
  assert.throws(
    () => validatePortableOutputDir(parse(projectRoot).root, { projectRoot, homeDir }),
    /filesystem root/,
  );
});

test("portable output refuses the user home directory and its ancestors", () => {
  assert.throws(
    () => validatePortableOutputDir(homeDir, { projectRoot, homeDir }),
    /user home directory/,
  );
  assert.throws(
    () => validatePortableOutputDir(resolve(sep, "home"), { projectRoot, homeDir }),
    /user home directory/,
  );
});

test("project-relative portable output must stay inside the packaging namespace", () => {
  assert.throws(() => validatePortableOutputDir("build", { projectRoot, homeDir }), /dist-portable/);
  assert.throws(
    () => validatePortableOutputDir(join("desktop", "dist"), { projectRoot, homeDir }),
    /dist-portable/,
  );
});

test("portable output rejects empty values and requires a project root", () => {
  assert.throws(() => validatePortableOutputDir("   ", { projectRoot, homeDir }), /non-empty/);
  assert.throws(() => validatePortableOutputDir(undefined, { projectRoot, homeDir }), /non-empty/);
  assert.throws(() => validatePortableOutputDir("dist-portable/XArchive", {}), /requires a projectRoot/);
});

test("portable output allows an explicit absolute directory outside protected roots", () => {
  const external = resolve(sep, "srv", "xarchive-packages", "XArchive");
  assert.equal(validatePortableOutputDir(external, { projectRoot, homeDir }), external);
});

test("Windows build routes only .cmd/.bat launchers through the command shell", () => {
  // Windows batch 6 (2026-09-29): `npm` resolves to `npm.cmd`, a batch file
  // that `spawn(..., { shell: false })` rejects with EINVAL. Everything else
  // must keep `shell: false` so this fix cannot reintroduce cmd.exe argument
  // splitting for native driver executables (WQ-P0-WHITE).
  assert.deepEqual(spawnOptionsForCommand("npm.cmd", "win32"), { shell: true });
  assert.deepEqual(spawnOptionsForCommand("C:\\tools\\setup.bat", "win32"), { shell: true });
  assert.deepEqual(spawnOptionsForCommand("npm", "win32"), { shell: false });
  assert.deepEqual(spawnOptionsForCommand("msedgedriver.exe", "win32"), { shell: false });
  assert.deepEqual(spawnOptionsForCommand("tauri-driver", "win32"), { shell: false });
  assert.deepEqual(spawnOptionsForCommand("npm.cmd", "linux"), { shell: false });
  assert.deepEqual(spawnOptionsForCommand("npm", "linux"), { shell: false });
});

// ENG-09: a recursive copy does not honour .gitignore, so local secrets,
// databases, logs, and caches must be filtered out explicitly.
test("package file filter drops local secrets, databases, logs, and caches", () => {
  const files = [
    "manifest.json",
    "xarchive-desktop.exe",
    ".env",
    ".env.local",
    "config/archive.sqlite3",
    "config/archive.sqlite3-wal",
    "logs/xarchive-1.log",
    "__pycache__/module.cpython-312.pyc",
    "nested/__pycache__/module.cpython-312.pyc",
    "sidecar/worker.pyc",
    ".pytest_cache/CACHEDIR.TAG",
    "node_modules/pkg/index.js",
    "target/release/artifact.bin",
    "test-artifacts/session.png",
    ".DS_Store",
    "Thumbs.db",
  ];
  const kept = filterPackageFiles(files);
  assert.deepEqual(kept, ["manifest.json", "xarchive-desktop.exe"]);
});

test("package file filter keeps legitimate nested component files", () => {
  const files = [
    "manifest.json",
    "src/content.js",
    "src/nested/deep/value.json",
    "sidecar/xarchive-downloader/xarchive-downloader.exe",
    "sidecar/gallery-dl/gallery-dl.exe",
  ];
  assert.deepEqual(filterPackageFiles(files), files);
});

test("package file filter keeps PyInstaller runtime extension modules", () => {
  // Windows batch 6: a global `*.pyd` exclusion stripped 7 C-extension modules
  // from the one-dir worker's `_internal` directory while the protocol smoke
  // still passed. A real HTTPS download needs `_ssl.pyd` at runtime, so `.pyd`
  // files — runtime binaries, not local artifacts — must ship; caches and
  // local files stay excluded.
  const files = [
    "sidecar/xarchive-downloader/xarchive-downloader.exe",
    "sidecar/xarchive-downloader/_internal/python312.dll",
    "sidecar/xarchive-downloader/_internal/_ssl.pyd",
    "sidecar/xarchive-downloader/_internal/_hashlib.pyd",
    "sidecar/gallery-dl/_internal/_lzma.pyd",
    "sidecar/xarchive-downloader/_internal/__pycache__/mod.cpython-312.pyc",
    "sidecar/xarchive-downloader/_internal/.env",
    "logs/xarchive-1.log",
  ];
  assert.deepEqual(filterPackageFiles(files), [
    "sidecar/xarchive-downloader/xarchive-downloader.exe",
    "sidecar/xarchive-downloader/_internal/python312.dll",
    "sidecar/xarchive-downloader/_internal/_ssl.pyd",
    "sidecar/xarchive-downloader/_internal/_hashlib.pyd",
    "sidecar/gallery-dl/_internal/_lzma.pyd",
  ]);
});

test("portable manifest version derives from the Tauri config", () => {
  // The manifest used to default to "unknown" because the release workflow
  // never sets PORTABLE_APP_VERSION; deriving from tauri.conf.json keeps
  // package-manifest.json in step with the version stamped into the PE.
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
  assert.match(portablePackageVersion(repoRoot), /^\d+\.\d+\.\d+/);
  assert.equal(
    portablePackageVersion(repoRoot, { PORTABLE_APP_VERSION: "9.9.9" }),
    "9.9.9",
  );
});

test("Native Host release tag is derived from the same product version", () => {
  // The installation manifest requires a `vMAJOR.MINOR.PATCH` release tag, so
  // the tag must come from the same source as the packaged version instead of a
  // second, independently defaulted literal.
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
  const configured = portablePackageVersion(repoRoot);
  assert.equal(portablePackageReleaseTag(repoRoot), `v${configured}`);
  assert.equal(
    portablePackageReleaseTag(repoRoot, { PORTABLE_APP_VERSION: "v0.2.0-pre.11" }),
    "v0.2.0-pre.11",
  );
  assert.equal(
    portablePackageReleaseTag(repoRoot, { PORTABLE_APP_VERSION: "0.2.0" }),
    "v0.2.0",
  );
});

test("portable manifest records the Native Host installation block", () => {
  const nativeHost = {
    name: "com.tw2tg.xarchive",
    executable: "native-host/xarchive-native-host.exe",
    manifest: "native-host/com.tw2tg.xarchive.json",
    registration: "windows-registry-or-user-native-messaging-host",
    allowed_origins: [],
  };
  const full = createManifest("full", "xarchive-desktop.exe", "0.2.0", nativeHost);
  assert.deepEqual(full.native_host, nativeHost);
  assert.equal(createManifest("core").native_host, null);
});

test("product version sources stay aligned", () => {
  // Windows batch 6 flagged the Cargo workspace 0.1.1 vs Tauri/PE/package
  // 0.1.0 split (CROSS_PLATFORM_REVIEW_REQUIRED): the workspace version is
  // canonical and every product-facing copy must match it.
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
  const read = (relativePath) => readFileSync(join(repoRoot, relativePath), "utf8");
  const workspaceVersion = read("Cargo.toml").match(
    /\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
  );
  assert.ok(workspaceVersion, "Cargo.toml [workspace.package] version not found");
  assert.equal(
    JSON.parse(read("desktop/src-tauri/tauri.conf.json")).version,
    workspaceVersion[1],
  );
  assert.equal(JSON.parse(read("package.json")).version, workspaceVersion[1]);
  assert.equal(JSON.parse(read("desktop/package.json")).version, workspaceVersion[1]);
  // The Extension ships as its own component package, so its manifest and npm
  // package must carry the same version as the desktop application.
  assert.equal(JSON.parse(read("extension/manifest.json")).version, workspaceVersion[1]);
  assert.equal(JSON.parse(read("extension/package.json")).version, workspaceVersion[1]);
  // The dashboard sidebar renders a placeholder version until `get_app_status`
  // answers; a stale literal there is the mismatch Windows batch 6 reported.
  const dashboardSource = read("desktop/src/main.jsx");
  assert.match(dashboardSource, new RegExp(`app_version: "${workspaceVersion[1]}"`));
});

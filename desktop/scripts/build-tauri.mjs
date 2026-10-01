// Resolve the release channel for a Tauri build and inject it into the
// backend before `tauri build` starts.
//
// `option_env!("XARCHIVE_RELEASE_CHANNEL")` is read at compile time, so the
// variable has to be present when the Rust crate is compiled. Setting it here
// keeps the channel decision in one place instead of duplicating it in every
// workflow and npm script.
//
// Resolution order:
//   1. An explicit `XARCHIVE_RELEASE_CHANNEL` already in the environment.
//   2. A validated release tag in `XARCHIVE_RELEASE_TAG`, e.g. `v0.2.1-pre1`.
//   3. `dev` for a local build.
//
// An unrecognizable tag fails instead of defaulting, because silently building
// a pre-release as `release` would hide exactly the diagnostics the pre-release
// exists to produce.

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const CHANNELS = new Set(["dev", "prerelease", "release"]);

// `windows-release.yml` validates this pattern before calling the build, and
// `pre-release.yml` only creates tags that match it.
const RELEASE_TAG_PATTERN = /^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

export function channelForTag(tag) {
  if (typeof tag !== "string" || tag.trim() === "") return "dev";
  const value = tag.trim();
  if (!RELEASE_TAG_PATTERN.test(value)) {
    throw new Error(`Unexpected release tag format: ${value}`);
  }
  // A tag that carries a pre-release identifier is a pre-release; a plain
  // three-part tag is a stable release.
  return /-/.test(value) ? "prerelease" : "release";
}

export function resolveChannel(env = process.env) {
  const explicit = (env.XARCHIVE_RELEASE_CHANNEL || "").trim().toLowerCase();
  if (explicit) {
    if (!CHANNELS.has(explicit)) {
      throw new Error(
        `Unknown XARCHIVE_RELEASE_CHANNEL "${explicit}"; expected dev, prerelease, or release`
      );
    }
    return explicit;
  }
  return channelForTag(env.XARCHIVE_RELEASE_TAG || "");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  let channel;
  try {
    channel = resolveChannel(process.env);
  } catch (error) {
    console.error(`release channel could not be resolved: ${error.message}`);
    process.exit(1);
  }
  process.env.XARCHIVE_RELEASE_CHANNEL = channel;
  console.log(`Building with XARCHIVE_RELEASE_CHANNEL=${channel}`);

  const desktopDir = fileURLToPath(new URL("..", import.meta.url));
  const tauriCli = fileURLToPath(new URL("../../node_modules/@tauri-apps/cli/tauri.js", import.meta.url));
  const result = spawnSync(process.execPath, [tauriCli, "build", ...process.argv.slice(2)], {
    cwd: desktopDir,
    env: process.env,
    stdio: "inherit",
  });

  if (result.error) throw result.error;
  process.exit(result.status === null ? 1 : result.status);
}

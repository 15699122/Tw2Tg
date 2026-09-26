import { join, normalize, parse, resolve, sep } from "node:path";

export const PORTABLE_PACKAGE_TYPES = new Set(["full", "core"]);

/// Only directories under this project-relative namespace may be removed by the
/// packaging script. Anything else requires an explicit absolute output path.
export const PORTABLE_OUTPUT_NAMESPACE = "dist-portable";

/// Compare two resolved paths segment by segment, independent of the platform
/// separator and of `relative()` returning an absolute path for a different
/// root. `mode` selects the direction:
///   "inside"   -> `candidate` is `parent` or lives underneath it
///   "ancestor" -> `candidate` is `parent` or contains it
function pathRelation(candidate, parent, mode) {
  if (candidate === parent) {
    return true;
  }
  const candidateParts = resolve(candidate).split(sep);
  const parentParts = resolve(parent).split(sep);
  const longer = mode === "inside" ? candidateParts : parentParts;
  const shorter = mode === "inside" ? parentParts : candidateParts;
  if (longer.length <= shorter.length) {
    return false;
  }
  return shorter.every((part, index) => part === longer[index]);
}

/// Files that must never be shipped in a portable package even when they exist
/// inside a copied component directory. A recursive copy does not honour
/// `.gitignore`, so a local `.env`, credential file, or test fixture under
/// `extension/` or `sidecar/` would otherwise reach the release artifact.
export const EXCLUDED_PACKAGE_PATTERNS = [
  ".env",
  ".env.*",
  "*.sqlite3",
  "*.sqlite3-*",
  "*.log",
  "__pycache__",
  "*.pyc",
  "*.pyo",
  "*.pyd",
  ".pytest_cache",
  ".ruff_cache",
  "node_modules",
  "target",
  "test-artifacts",
  ".DS_Store",
  "Thumbs.db",
];

/// Compile one glob pattern into an anchored regular expression.
///
/// `*` matches any run of characters, `?` matches one, and everything else is
/// literal. This is applied to the whole relative path so patterns such as
/// `*.sqlite3-*` also catch a directory component.
function globToRegExp(pattern) {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  const body = escaped.replace(/\*/g, ".*").replace(/\?/g, ".");
  return new RegExp(`^${body}$`);
}

function isExcludedPackageFile(relativePath) {
  const normalized = relativePath.replace(/\\/g, "/");
  const parts = normalized.split("/");
  const candidates = [normalized, ...parts];
  return EXCLUDED_PACKAGE_PATTERNS.some((pattern) => {
    const regex = globToRegExp(pattern);
    // A directory name anywhere in the path excludes the whole subtree, and a
    // pattern such as `*.sqlite3-*` matches the full relative path.
    return candidates.some((candidate) => regex.test(candidate));
  });
}

/// Filter a recursive file list, dropping anything that must not ship.
export function filterPackageFiles(relativePaths) {
  return relativePaths.filter((relativePath) => !isExcludedPackageFile(relativePath));
}

export function validatePackageType(packageType) {
  if (!PORTABLE_PACKAGE_TYPES.has(packageType)) {
    throw new Error(`PORTABLE_PACKAGE_TYPE must be full or core, got: ${packageType}`);
  }
  return packageType;
}

/// Refuse to treat an arbitrary directory as a deletable packaging output.
///
/// The build script removes the resolved output directory before assembling a
/// package, so an operator-supplied `PORTABLE_OUTPUT_DIR` must never be able to
/// point at the project root, one of its ancestors, the user home directory, a
/// filesystem root, or an unrelated absolute location.
export function validatePortableOutputDir(outputDir, { projectRoot, homeDir } = {}) {
  if (typeof outputDir !== "string" || outputDir.trim() === "") {
    throw new Error("PORTABLE_OUTPUT_DIR must be a non-empty path");
  }
  if (projectRoot === undefined) {
    throw new Error("validatePortableOutputDir requires a projectRoot");
  }
  const resolvedRoot = resolve(projectRoot);
  const resolvedOutput = resolve(resolvedRoot, outputDir);

  if (resolvedOutput === parse(resolvedOutput).root) {
    throw new Error(`Refusing to use a filesystem root as PORTABLE_OUTPUT_DIR: ${resolvedOutput}`);
  }
  if (resolvedOutput === resolvedRoot) {
    throw new Error(`Refusing to use the project root as PORTABLE_OUTPUT_DIR: ${resolvedOutput}`);
  }
  if (pathRelation(resolvedOutput, resolvedRoot, "ancestor")) {
    throw new Error(
      `Refusing to use an ancestor of the project root as PORTABLE_OUTPUT_DIR: ${resolvedOutput}`,
    );
  }
  // Home-directory protection is opt-in so that a normal portable output under
  // the user profile stays valid, while the build script passes the real home
  // directory to reject that location explicitly.
  if (homeDir !== undefined) {
    const resolvedHome = resolve(homeDir);
    if (pathRelation(resolvedOutput, resolvedHome, "ancestor")) {
      throw new Error(
        `Refusing to use the user home directory or its ancestor as PORTABLE_OUTPUT_DIR: ${resolvedOutput}`,
      );
    }
  }
  const normalizedOutput = normalize(resolvedOutput);
  // Only a path that really lives inside the project root is subject to the
  // packaging namespace rule. An explicit absolute output elsewhere (for
  // example `D:\packages\XArchive` on Windows or `/srv/...` on Linux) stays
  // valid as long as it is not a protected root, the project root, or one of
  // its ancestors.
  if (pathRelation(normalizedOutput, resolvedRoot, "inside")) {
    const projectRelative = normalizedOutput.slice(resolvedRoot.length + 1);
    if (!projectRelative.split(sep).includes(PORTABLE_OUTPUT_NAMESPACE)) {
      throw new Error(
        `Project-relative PORTABLE_OUTPUT_DIR must be inside ${PORTABLE_OUTPUT_NAMESPACE}/: ${normalizedOutput}`,
      );
    }
  }
  return normalizedOutput;
}

export function packageDirectories(packageType) {
  validatePackageType(packageType);
  return [
    "config",
    "cache/staging",
    "cache/downloads",
    "cache/runtime",
    "logs",
    ...(packageType === "full" ? ["sidecar/gallery-dl"] : []),
    "sidecar/xarchive-downloader",
    "sidecar/aria2",
  ];
}

export function componentPlan(projectRoot, outputRoot, packageType) {
  validatePackageType(packageType);

  // Component plan uses a three-state presence flag:
  //  - "required":  must exist in the source tree or packaging fails.
  //  - "optional":  copied when present, skipped when absent.
  //  - "excluded":  never copied into the package, regardless of source.
  const isFull = packageType === "full";
  return [
    // worker: always bundled (one-dir layout, matches config.rs default)
    [join(projectRoot, "sidecar", "xarchive-downloader"), join(outputRoot, "sidecar", "xarchive-downloader"), "required"],
    // aria2: copied when a local bundled artifact is available
    [join(projectRoot, "sidecar", "aria2"), join(outputRoot, "sidecar", "aria2"), "optional"],
    // gallery-dl: bundled in Full, explicitly excluded from Core
    [join(projectRoot, "sidecar", "gallery-dl"), join(outputRoot, "sidecar", "gallery-dl"), isFull ? "required" : "excluded"],
  ];
}

export function createManifest(packageType, executable = "xarchive-desktop.exe", version = "unknown") {
  validatePackageType(packageType);
  return {
    schema_version: 1,
    package_type: packageType,
    platform: "windows-x64",
    desktop: { executable, version },
    sidecar: {
      worker: "sidecar/xarchive-downloader",
      gallery_dl: packageType === "full" ? "sidecar/gallery-dl/gallery-dl.exe" : null,
      gallery_dl_bundled: packageType === "full",
    },
    extension: {
      directory: "extension",
      bundled: packageType === "full",
      user_importable: false,
    },
  };
}
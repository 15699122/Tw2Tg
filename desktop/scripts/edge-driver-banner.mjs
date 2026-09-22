// Copyright (c) 2026 Tw2Tg contributors.
// SPDX-License-Identifier: MIT

/**
 * Microsoft Edge WebDriver banner compatibility helper.
 *
 * This is Linux-verifiable test infrastructure only. It models the two
 * discovery contracts that matter for the current Windows blocker:
 *
 * - installed `@wdio/tauri-service@1.4.0` accepts only `MSEdgeDriver x.y`;
 * - the project Windows preflight accepts both `MSEdgeDriver x.y` and the
 *   current Microsoft executable banner `Microsoft Edge WebDriver x.y`.
 *
 * No UI assertion, Tauri capability, driver version, download policy, or
 * production behavior is changed by this module.
 */

export const WDIO_TAURI_SERVICE_VERSION = "1.4.0";
export const WDIO_SERVICE_EDGE_DRIVER_PATTERN = /MSEdgeDriver ([\d.]+)/;
export const PREFLIGHT_EDGE_DRIVER_PATTERN = /(MSEdgeDriver|Microsoft Edge WebDriver)\s+([\d.]+)/;

export function parseServiceEdgeDriverVersion(banner) {
  if (typeof banner !== "string") {
    return null;
  }
  const match = banner.match(WDIO_SERVICE_EDGE_DRIVER_PATTERN);
  return match ? match[1] : null;
}

export function parsePreflightEdgeDriverVersion(banner) {
  if (typeof banner !== "string") {
    return null;
  }
  const match = banner.match(PREFLIGHT_EDGE_DRIVER_PATTERN);
  return match ? match[2] : null;
}

export function serviceAcceptsEdgeDriverBanner(banner) {
  return parseServiceEdgeDriverVersion(banner) !== null;
}

export function preflightAcceptsEdgeDriverBanner(banner, expectedVersion = "") {
  const version = parsePreflightEdgeDriverVersion(banner);
  if (version === null) {
    return false;
  }
  if (typeof expectedVersion !== "string" || expectedVersion.trim() === "") {
    return true;
  }
  return banner.includes(expectedVersion.trim());
}

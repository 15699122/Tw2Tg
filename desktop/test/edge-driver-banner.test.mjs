// Copyright (c) 2026 Tw2Tg contributors.
// SPDX-License-Identifier: MIT

import test from "node:test";
import assert from "node:assert/strict";

import {
  PREFLIGHT_EDGE_DRIVER_PATTERN,
  WDIO_SERVICE_EDGE_DRIVER_PATTERN,
  parsePreflightEdgeDriverVersion,
  parseServiceEdgeDriverVersion,
  preflightAcceptsEdgeDriverBanner,
  serviceAcceptsEdgeDriverBanner,
} from "../scripts/edge-driver-banner.mjs";

const LEGACY_BANNER = "MSEdgeDriver 152.0.4191.66";
const CURRENT_MICROSOFT_BANNER = "Microsoft Edge WebDriver 152.0.4191.66";

test("installed WDIO service only accepts the legacy MSEdgeDriver banner", () => {
  assert.equal(WDIO_SERVICE_EDGE_DRIVER_PATTERN.source, "MSEdgeDriver ([\\d.]+)");
  assert.equal(parseServiceEdgeDriverVersion(LEGACY_BANNER), "152.0.4191.66");
  assert.equal(serviceAcceptsEdgeDriverBanner(LEGACY_BANNER), true);
  assert.equal(parseServiceEdgeDriverVersion(CURRENT_MICROSOFT_BANNER), null);
  assert.equal(serviceAcceptsEdgeDriverBanner(CURRENT_MICROSOFT_BANNER), false);
  assert.equal(serviceAcceptsEdgeDriverBanner(""), false);
  assert.equal(serviceAcceptsEdgeDriverBanner(null), false);
});

test("Windows preflight accepts both legacy and current Microsoft banners", () => {
  assert.match("MSEdgeDriver 152.0.4191.66", PREFLIGHT_EDGE_DRIVER_PATTERN);
  assert.match("Microsoft Edge WebDriver 152.0.4191.66", PREFLIGHT_EDGE_DRIVER_PATTERN);
  assert.equal(parsePreflightEdgeDriverVersion(LEGACY_BANNER), "152.0.4191.66");
  assert.equal(parsePreflightEdgeDriverVersion(CURRENT_MICROSOFT_BANNER), "152.0.4191.66");
  assert.equal(preflightAcceptsEdgeDriverBanner(CURRENT_MICROSOFT_BANNER, "152.0.4191.66"), true);
  assert.equal(preflightAcceptsEdgeDriverBanner(CURRENT_MICROSOFT_BANNER, "153.0.4234.48"), false);
  assert.equal(preflightAcceptsEdgeDriverBanner("", "152.0.4191.66"), false);
});

test("banner compatibility remains diagnostic-only and does not claim a session", () => {
  // A recognized preflight banner is necessary evidence, not proof that the
  // installed WDIO service can create a Windows WebDriver session.
  assert.equal(preflightAcceptsEdgeDriverBanner(CURRENT_MICROSOFT_BANNER, "152.0.4191.66"), true);
  assert.equal(serviceAcceptsEdgeDriverBanner(CURRENT_MICROSOFT_BANNER), false);
});

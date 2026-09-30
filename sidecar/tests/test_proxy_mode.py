"""Proxy mode contract for the Sidecar child environment.

The Desktop sends ``XARCHIVE_PROXY_MODE`` and, in manual mode,
``XARCHIVE_PROXY``. These tests pin what each mode does to the environment a
gallery-dl child actually receives, because a ``direct`` promise that only
clears the Rust side would be silently broken by an inherited variable here.
"""

from __future__ import annotations

import pytest

from xarchive_downloader.extraction import (
    PROXY_ENVIRONMENT_KEYS,
    ExtractionConfig,
    spawn_env,
)

PROXY = "http://alice:s3cret@proxy.example:8080"


@pytest.fixture
def inherited(monkeypatch: pytest.MonkeyPatch) -> None:
    for key in PROXY_ENVIRONMENT_KEYS:
        monkeypatch.setenv(key, "http://inherited.example:3128")


def test_direct_removes_every_inherited_proxy_variable(inherited: None) -> None:
    env = spawn_env(None, "direct")
    assert env is not None, "direct must pass an explicit environment"
    for key in PROXY_ENVIRONMENT_KEYS:
        assert key not in env, f"direct must remove {key}"


def test_direct_still_removes_a_stored_value(inherited: None) -> None:
    env = spawn_env(PROXY, "direct")
    assert env is not None
    for key in PROXY_ENVIRONMENT_KEYS:
        assert key not in env, "a stored value must not reach a direct child"


def test_direct_removes_variables_that_ignore_case(inherited: None) -> None:
    monkeypatch_extra = {"Http_Proxy": "http://mixed.example:1"}
    import os

    original = dict(os.environ)
    try:
        os.environ.update(monkeypatch_extra)
        env = spawn_env(None, "direct")
        assert env is not None
        assert "Http_Proxy" not in env
    finally:
        os.environ.clear()
        os.environ.update(original)


def test_manual_sets_the_value_for_every_spelling(inherited: None) -> None:
    env = spawn_env(PROXY, "manual")
    assert env is not None
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"):
        assert env[key] == PROXY, f"manual must set {key}"


def test_system_leaves_the_inherited_environment_untouched(inherited: None) -> None:
    # gallery-dl must be able to discover the platform proxy itself.
    assert spawn_env(None, "system") is None
    assert spawn_env(PROXY, "system") is None, "system must not pin a value"


def test_an_unknown_mode_is_treated_as_system(inherited: None) -> None:
    assert spawn_env(None, "pac") is None
    assert spawn_env(None, None) is None


def test_the_configuration_carries_the_mode_to_the_runner() -> None:
    assert ExtractionConfig().proxy_mode == "system"
    assert ExtractionConfig(proxy_mode="direct").proxy_mode == "direct"


def test_the_proxy_environment_keys_match_the_rust_contract() -> None:
    # The Rust side removes exactly this set; a divergence would leave one
    # variable behind on the child.
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"):
        assert key in PROXY_ENVIRONMENT_KEYS
    assert len(PROXY_ENVIRONMENT_KEYS) == len(set(PROXY_ENVIRONMENT_KEYS))
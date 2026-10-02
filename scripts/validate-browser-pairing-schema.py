#!/usr/bin/env python3
"""Validate browser-pairing JSON Schemas and representative fixtures.

Requires the development-only `jsonschema` Python package (Draft 2020-12).
This is intentionally not a Desktop/runtime dependency.
"""

import json
import sys
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
except ImportError as error:
    raise SystemExit(
        "Missing development validator. Install with: python -m pip install 'jsonschema>=4.18,<5'"
    ) from error


ROOT = Path(__file__).resolve().parents[1]
SCHEMAS = ROOT / "shared" / "protocol-schema"


def validate(name: str, accepted: list[dict], rejected: list[dict]) -> None:
    schema_path = SCHEMAS / f"browser-pairing-{name}.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    for index, instance in enumerate(accepted):
        errors = list(validator.iter_errors(instance))
        if errors:
            raise AssertionError(f"{schema_path.name}: valid fixture {index} rejected: {errors[0].message}")
    for index, instance in enumerate(rejected):
        if validator.is_valid(instance):
            raise AssertionError(f"{schema_path.name}: invalid fixture {index} accepted")
    print(f"PASS {schema_path.name}: {len(accepted)} valid, {len(rejected)} invalid")


def main() -> None:
    request = {"protocol_version": 1, "request_id": "boot-1", "message_type": "bootstrap"}
    endpoint = {
        "protocol_version": 1,
        "request_id": "boot-1",
        "message_type": "bootstrap",
        "host": "127.0.0.1",
        "port": 43210,
        "path": "/",
        "runtime_instance_id": "runtime-1",
        "ticket": "a" * 64,
        "expires_in_ms": 30_000,
    }
    pairing_error = {
        "protocol_version": 1,
        "request_id": "boot-1",
        "message_type": "error",
        "error_code": "DESKTOP_UNAVAILABLE",
        "error_message": "Desktop is unavailable",
        "retryable": True,
    }
    authentication = {"protocol_version": 1, "message_type": "authenticate", "ticket": "b" * 64}
    auth_ok = {"protocol_version": 1, "message_type": "authentication_response", "authenticated": True}
    auth_failed = {
        "protocol_version": 1,
        "message_type": "authentication_response",
        "authenticated": False,
        "error_code": "AUTHENTICATION_FAILED",
    }

    validate("request", [request], [request | {"extra": True}, request | {"protocol_version": 2}])
    validate(
        "response",
        [endpoint, pairing_error],
        [endpoint | {"host": "::1"}, endpoint | {"expires_in_ms": 30_001}, endpoint | {"extra": True}],
    )
    validate("authentication", [authentication], [authentication | {"ticket": "A" * 64}])
    validate(
        "authentication-response",
        [auth_ok, auth_failed],
        [auth_ok | {"error_code": "AUTHENTICATION_FAILED"}, auth_failed | {"error_code": "WRONG"}],
    )


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, json.JSONDecodeError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        raise SystemExit(1) from error
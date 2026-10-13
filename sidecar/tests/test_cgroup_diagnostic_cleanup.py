"""Failure cleanup must close ownership without deleting an undrained group."""

import importlib.util
from pathlib import Path

import pytest


def diagnostic():
    path = Path(__file__).resolve().parents[2] / "scripts/validate-cgroup-containment.py"
    spec = importlib.util.spec_from_file_location("cgroup_diagnostic", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize("failure", ["drain", "wait", None])
def test_cleanup_closes_descriptor_and_preserves_failed_group(monkeypatch, failure):
    module = diagnostic()
    calls = []

    def drain(directory):
        calls.append(("drain", directory))
        if failure == "drain":
            raise TimeoutError("drain failed")

    class Process:
        def wait(self, timeout):
            calls.append(("wait", timeout))
            if failure == "wait":
                raise TimeoutError("wait failed")

    class Group:
        def rmdir(self):
            calls.append(("remove",))

    monkeypatch.setattr(module, "kill_and_drain", drain)
    monkeypatch.setattr(module.os, "close", lambda fd: calls.append(("close", fd)))
    if failure is None:
        module.cleanup_group(123, Group(), Process())
        assert calls == [("drain", 123), ("wait", 5), ("close", 123), ("remove",)]
    else:
        with pytest.raises(TimeoutError):
            module.cleanup_group(123, Group(), Process())
        assert calls[-1] == ("close", 123)
        assert ("remove",) not in calls
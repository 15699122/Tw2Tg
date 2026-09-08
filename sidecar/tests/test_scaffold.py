from xarchive_downloader import main


def test_package_exposes_worker_entrypoint() -> None:
    assert callable(main)
from pathlib import Path

from PyInstaller.utils.hooks import collect_submodules


project_root = Path(SPECPATH).parent.parent
source_root = project_root / "src"

hiddenimports = collect_submodules("xarchive_downloader")

a = Analysis(
    [str(Path(SPECPATH) / "entrypoint_v2.py")],
    pathex=[str(source_root)],
    binaries=[],
    datas=[],
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    [],
    name="xarchive-downloader",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,
    exclude_binaries=True,
)

# one-dir layout: build a self-contained directory
# `dist/xarchive-downloader/xarchive-downloader.exe` so the artifact matches
# the layout consumed by `.github/workflows/windows-worker-artifact.yml` and
# `desktop/scripts/build-portable-windows.mjs`.
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="xarchive-downloader",
)
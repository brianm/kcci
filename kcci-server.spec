# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller spec for KCCI server sidecar."""

import sys
from pathlib import Path

# Get the project root
project_root = Path(SPECPATH)
venv_site_packages = project_root / '.venv' / 'lib' / 'python3.11' / 'site-packages'

a = Analysis(
    ['kcci-server.py'],
    pathex=[str(project_root / 'src')],
    binaries=[
        # Include sqlite_vec native extension
        (str(venv_site_packages / 'sqlite_vec' / 'vec0.dylib'), 'sqlite_vec'),
    ],
    datas=[
        # Include Flask templates
        (str(project_root / 'src' / 'kcci' / 'templates'), 'kcci/templates'),
        # Include pre-exported ONNX model
        (str(project_root / 'src-tauri' / 'binaries' / 'onnx-model'), 'onnx-model'),
    ],
    hiddenimports=[
        'kcci',
        'kcci.web',
        'kcci.db',
        'kcci.embed',
        'kcci.sync',
        'kcci.enrich',
        'kcci.webarchive',
        'waitress',
        'flask',
        'jinja2',
        'markupsafe',
        'markdown',
        'sqlite_vec',
        'onnxruntime',
        'tokenizers',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='kcci-server',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,  # Need console for stdout communication with Tauri
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

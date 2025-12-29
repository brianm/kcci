#!/usr/bin/env python3
"""Entrypoint for PyInstaller-bundled KCCI server.

This is the entry point for the standalone desktop app.
It sets up the app-specific data directory and starts the Flask server.
"""

import os
import socket
import sys
from pathlib import Path


def get_app_data_dir() -> Path:
    """Get the app-specific data directory."""
    # macOS Application Support directory
    return Path.home() / "Library" / "Application Support" / "KCCI"


def find_available_port() -> int:
    """Find a random available port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def main():
    # Set environment variable for app-specific data directory
    # This is read by db.py to determine where to store data
    data_dir = get_app_data_dir()
    data_dir.mkdir(parents=True, exist_ok=True)
    os.environ["KCCI_DATA_DIR"] = str(data_dir)

    # Find available port
    port = find_available_port()

    # Print port for Tauri to read (must be this exact format)
    print(f"PORT:{port}", flush=True)

    # Import and run the Flask app
    from kcci.web import app
    from waitress import serve

    # Run the server (blocking)
    serve(app, host="127.0.0.1", port=port)


if __name__ == "__main__":
    main()

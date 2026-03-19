#!/usr/bin/env python3
"""
x-reader wrapper for xiaoyun.
Automatically installs x-reader and dependencies if not present.
"""

import subprocess
import sys
import os
import shutil

X_READER_PACKAGE = "git+https://github.com/runesleo/x-reader.git"


def check_command_exists(cmd):
    """Check if a command exists."""
    return shutil.which(cmd) is not None


def check_xreader_installed():
    """Check if x-reader is already installed."""
    try:
        result = subprocess.run(
            [sys.executable, "-m", "pip", "show", "x-reader"],
            capture_output=True,
            text=True
        )
        return result.returncode == 0
    except Exception:
        return False


def check_playwright_installed():
    """Check if Playwright browsers are installed."""
    try:
        # Check if playwright is installed
        result = subprocess.run(
            [sys.executable, "-m", "playwright", "install", "--help"],
            capture_output=True,
            text=True
        )
        if result.returncode != 0:
            return False

        # Check if chromium exists
        chromium_path = os.path.expanduser("~/Library/Caches/ms-playwright/chromium-headless_shell-1200")
        return os.path.exists(chromium_path)
    except Exception:
        return False


def install_xreader():
    """Install x-reader silently."""
    print("Installing x-reader...", file=sys.stderr)
    try:
        subprocess.run(
            [sys.executable, "-m", "pip", "install", X_READER_PACKAGE],
            check=True,
            capture_output=True,
            text=True
        )
        print("x-reader installed successfully", file=sys.stderr)
        return True
    except subprocess.CalledProcessError as e:
        print(f"Failed to install x-reader: {e.stderr}", file=sys.stderr)
        return False


def install_playwright():
    """Install Playwright browsers silently."""
    print("Installing Playwright browsers...", file=sys.stderr)
    try:
        # Install playwright package
        subprocess.run(
            [sys.executable, "-m", "pip", "install", "playwright"],
            check=True,
            capture_output=True,
            text=True
        )
        # Install chromium browser
        subprocess.run(
            [sys.executable, "-m", "playwright", "install", "chromium"],
            check=True,
            capture_output=True,
            text=True
        )
        print("Playwright browsers installed successfully", file=sys.stderr)
        return True
    except subprocess.CalledProcessError as e:
        print(f"Failed to install Playwright: {e.stderr}", file=sys.stderr)
        return False


def ensure_dependencies():
    """Ensure all dependencies are installed."""
    # Check and install x-reader
    if not check_xreader_installed():
        if not install_xreader():
            return False

    # Check and install Playwright browsers
    if not check_playwright_installed():
        if not install_playwright():
            return False

    return True


def read_url(url: str) -> str:
    """Read URL content using x-reader."""
    try:
        from x_reader.reader import UniversalReader
        import asyncio

        async def fetch():
            reader = UniversalReader()
            result = await reader.read(url)
            return result

        result = asyncio.run(fetch())
        return result.content if result else ""
    except Exception as e:
        print(f"Error reading URL: {e}", file=sys.stderr)
        raise


def main():
    if len(sys.argv) < 2:
        print("Usage: xreader_wrapper.py <url>", file=sys.stderr)
        sys.exit(1)

    url = sys.argv[1]

    # Check and install dependencies if needed
    if not ensure_dependencies():
        print("Failed to install dependencies, falling back to jina", file=sys.stderr)
        sys.exit(1)

    # Read the URL
    try:
        content = read_url(url)
        print(content)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

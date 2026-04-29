#!/usr/bin/env python3
import sys

from markitdown import MarkItDown


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: openwiki-markitdown <file>", file=sys.stderr)
        return 2

    result = MarkItDown().convert(sys.argv[1])
    text = (result.text_content or "").strip()
    if not text:
        print("Converted document is empty", file=sys.stderr)
        return 3

    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

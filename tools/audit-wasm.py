#!/usr/bin/env python3
"""Minimal WebAssembly export audit used by the release build.

No third-party parser is required, which keeps the production surface gate reproducible.
"""
from __future__ import annotations
import argparse
from pathlib import Path


def uleb(data: bytes, i: int) -> tuple[int, int]:
    value = 0
    shift = 0
    while True:
        if i >= len(data):
            raise ValueError("truncated uleb")
        b = data[i]
        i += 1
        value |= (b & 0x7F) << shift
        if b < 0x80:
            return value, i
        shift += 7
        if shift > 63:
            raise ValueError("uleb too large")


def exports(data: bytes) -> list[str]:
    if data[:8] != b"\x00asm\x01\x00\x00\x00":
        raise ValueError("not a Wasm v1 module")
    out: list[str] = []
    i = 8
    while i < len(data):
        section_id = data[i]
        i += 1
        size, i = uleb(data, i)
        end = i + size
        if end > len(data):
            raise ValueError("truncated section")
        if section_id == 7:
            count, j = uleb(data, i)
            for _ in range(count):
                n, j = uleb(data, j)
                name = data[j:j+n].decode("utf-8")
                j += n
                if j >= end:
                    raise ValueError("truncated export descriptor")
                j += 1  # kind
                _, j = uleb(data, j)  # index
                out.append(name)
            if j != end:
                raise ValueError("unexpected trailing export data")
        i = end
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("wasm")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--backend", action="store_true")
    mode.add_argument("--frontend", action="store_true")
    args = ap.parse_args()
    data = Path(args.wasm).read_bytes()
    names = exports(data)
    app_methods = [n for n in names if n.startswith(("canister_update ", "canister_query ", "canister_composite_query "))]
    if args.backend:
        if app_methods:
            raise SystemExit(f"production backend unexpectedly exports application methods: {app_methods}")
        forbidden = [b"debug_", b"debug_poll_once", b"debug_funding_once", b"debug_subscription"]
        for needle in forbidden:
            if needle in data:
                raise SystemExit(f"production backend contains debug marker {needle!r}")
    else:
        if "canister_query http_request" not in names:
            raise SystemExit("frontend does not export certified http_request query")
        debug = [n for n in app_methods if "debug" in n.lower()]
        if debug:
            raise SystemExit(f"frontend unexpectedly exports debug methods: {debug}")
    print(f"{args.wasm}: export audit passed ({len(names)} exports)")


if __name__ == "__main__":
    main()

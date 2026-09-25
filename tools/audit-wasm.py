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
    # ic-cdk-timers exports this reserved callback for the global timer. It is
    # not an application method; reject every other query/update export.
    internal_timer = "canister_update <ic-cdk internal> timer_executor"
    app_methods = [
        n for n in names
        if n.startswith(("canister_update ", "canister_query ", "canister_composite_query "))
        and n != internal_timer
    ]
    if args.backend:
        expected = ["canister_query get_pricing"]
        if app_methods != expected:
            raise SystemExit(f"production backend application surface is {app_methods}, expected {expected}")
        # The IC system import `debug_print` is used for exceptional logs.
        forbidden = [b"debug_poll_once", b"debug_funding_once", b"debug_pricing_once",
                     b"debug_start_schedulers", b"debug_timer_count",
                     b"debug_subscription", b"debug_global_subscription", b"debug_state"]
        for needle in forbidden:
            if needle in data:
                raise SystemExit(f"production backend contains debug marker {needle!r}")
    else:
        expected = ["canister_query http_request"]
        if app_methods != expected:
            raise SystemExit(f"frontend application surface is {app_methods}, expected {expected}")
        if b"http_request_update" in data:
            raise SystemExit("frontend contains removed HTTP update proxy marker")
        backend_principal = b"eo6ei-gaaaa-aaaar-qchra-cai"
        if backend_principal not in data:
            raise SystemExit("frontend does not contain the permanent backend principal")
        for marker in [b"PUBLIC_CANISTER_ID:event_horizon", b"ic_env", b"IC_ROOT_KEY"]:
            if marker in data:
                raise SystemExit(f"frontend contains removed runtime-discovery marker {marker!r}")
    print(f"{args.wasm}: export audit passed ({len(names)} exports)")


if __name__ == "__main__":
    main()

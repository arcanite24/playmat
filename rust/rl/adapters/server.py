"""Standalone CLI server for running any Playmat external AI adapter over TCP or stdio.

Examples:
    python3 -m adapters.server --spec jev:typesafe/jev-1.13 --port 18500
    python3 -m adapters.server --spec llm:openrouter/anthropic/claude-opus-4.6 --port 18501
    python3 -m adapters.server --spec open:forge-heuristic --stdio
"""

from __future__ import annotations

import argparse
import json
import signal
import sys

from . import build_adapter
from .base import serve_stdio, start_tcp_server


def main() -> None:
    ap = argparse.ArgumentParser(description="Serve a Playmat external AI adapter (playmat-ext-v1 NDJSON).")
    ap.add_argument("--spec", required=True, help="Adapter spec (jev:..., llm:..., open:..., onnx:..., python:..., http://..., mcp:...)")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=18500)
    ap.add_argument("--stdio", action="store_true", help="Serve on stdin/stdout instead of TCP (for ext:cmd:...)")
    ap.add_argument("--base-url", default=None, help="Optional endpoint/base URL override for Jev or LLM providers")
    ap.add_argument("--reasoning-effort", default=None, help="Optional reasoning effort (low, medium, high) for LLM providers")
    ap.add_argument("--with-value", action="store_true", help="Ask Jev for a secondary board-evaluation score")
    ap.add_argument("--telemetry-out", default=None, help="Write JSON telemetry summary on exit")
    args = ap.parse_args()

    adapter = build_adapter(
        args.spec,
        base_url=args.base_url,
        reasoning_effort=args.reasoning_effort,
        with_value=args.with_value,
    )

    if args.stdio:
        try:
            serve_stdio(adapter)
        finally:
            if args.telemetry_out:
                with open(args.telemetry_out, "w", encoding="utf-8") as f:
                    json.dump(adapter.telemetry.summary(), f, indent=2)
        return

    srv, _, port = start_tcp_server(adapter, host=args.host, port=args.port)
    obs_suffix = "?obs=1" if adapter.needs_obs else ""
    print(f"ADAPTER_READY name={adapter.name} endpoint=ext:tcp://{args.host}:{port}{obs_suffix}", flush=True)

    stop = False

    def _handle_sig(_signum, _frame):
        nonlocal stop
        stop = True
        srv.shutdown()

    signal.signal(signal.SIGINT, _handle_sig)
    signal.signal(signal.SIGTERM, _handle_sig)

    try:
        while not stop:
            signal.pause() if hasattr(signal, "pause") else srv.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        srv.shutdown()
        srv.server_close()
        summary = adapter.telemetry.summary()
        print("ADAPTER_TELEMETRY " + json.dumps(summary), file=sys.stderr, flush=True)
        if args.telemetry_out:
            with open(args.telemetry_out, "w", encoding="utf-8") as f:
                json.dump(summary, f, indent=2)


if __name__ == "__main__":
    main()

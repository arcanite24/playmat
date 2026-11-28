"""Adapter registry and factory for Playmat external AI benchmarking.

Spec syntax:
  - `jev:<model>`                    -> JevAdapter (OpenRouter Decisions API, e.g. `jev:typesafe/jev-1.13`, `jev:mock`)
  - `llm:<provider>/<model>`         -> LlmAdapter (e.g. `llm:openrouter/anthropic/claude-opus-4.6`, `llm:ollama/qwen3:32b`)
  - `open:forge-heuristic`           -> ForgeHeuristicAdapter (`forge-heuristic`, `greedy-aggro`, `greedy-control`)
  - `onnx:<path.onnx>`               -> OnnxPolicyAdapter (requires ?obs=1)
  - `python:<module:ClassOrFunc>`    -> PythonPluginAdapter (MageZero, MTG-Causal-RL, custom policies)
  - `http://<url>` / `https://<url>` -> HttpAgentAdapter
  - `mcp:<command>`                  -> McpBridgeAdapter (mage-bench-compatible MCP stdio bridge)
"""

from __future__ import annotations

from typing import Optional

from .base import Adapter
from .gemini_decision import DEFAULT_GEMINI_DECISION_MODEL, GeminiDecisionAdapter
from .jev import DEFAULT_JEV_MODEL, JevAdapter
from .llm import LlmAdapter
from .open_ais import (
    ForgeHeuristicAdapter,
    HttpAgentAdapter,
    McpBridgeAdapter,
    OnnxPolicyAdapter,
    PythonPluginAdapter,
)

__all__ = [
    "Adapter",
    "GeminiDecisionAdapter",
    "JevAdapter",
    "LlmAdapter",
    "ForgeHeuristicAdapter",
    "OnnxPolicyAdapter",
    "PythonPluginAdapter",
    "HttpAgentAdapter",
    "McpBridgeAdapter",
    "build_adapter",
    "is_adapter_spec",
]

ADAPTER_PREFIXES = ("gemini-decision:", "decision:", "jev:", "llm:", "open:", "onnx:", "python:", "http://", "https://", "mcp:")


def is_adapter_spec(spec: str) -> bool:
    return spec.startswith(ADAPTER_PREFIXES)


def build_adapter(
    spec: str,
    *,
    base_url: Optional[str] = None,
    reasoning_effort: Optional[str] = None,
    with_value: bool = False,
) -> Adapter:
    if spec.startswith("gemini-decision:") or spec.startswith("decision:"):
        model = spec.split(":", 1)[1].strip() or DEFAULT_GEMINI_DECISION_MODEL
        return GeminiDecisionAdapter(model=model, endpoint=base_url, with_value=with_value)

    if spec.startswith("jev:"):
        model = spec[len("jev:") :].strip() or DEFAULT_JEV_MODEL
        return JevAdapter(model=model, endpoint=base_url, with_value=with_value)

    if spec.startswith("llm:"):
        rest = spec[len("llm:") :].strip()
        if rest == "mock":
            return LlmAdapter("mock", "mock")
        if "/" not in rest:
            raise ValueError(f"llm: spec must be 'llm:<provider>/<model>', got {spec!r}")
        provider, model = rest.split("/", 1)
        return LlmAdapter(provider, model, base_url=base_url, reasoning_effort=reasoning_effort)

    if spec.startswith("open:"):
        style = spec[len("open:") :].strip() or "forge-heuristic"
        if style not in ("forge-heuristic", "greedy-aggro", "greedy-control"):
            raise ValueError(f"Unknown open heuristic {style!r} (forge-heuristic, greedy-aggro, greedy-control)")
        return ForgeHeuristicAdapter(style=style)

    if spec.startswith("onnx:"):
        path = spec[len("onnx:") :].strip()
        return OnnxPolicyAdapter(path)

    if spec.startswith("python:"):
        rest = spec[len("python:") :].strip()
        needs_obs = False
        if rest.endswith("?obs=1"):
            needs_obs = True
            rest = rest[: -len("?obs=1")]
        return PythonPluginAdapter(rest, needs_obs=needs_obs)

    if spec.startswith("http://") or spec.startswith("https://"):
        needs_obs = "obs=1" in spec
        return HttpAgentAdapter(spec, needs_obs=needs_obs)

    if spec.startswith("mcp:"):
        cmd = spec[len("mcp:") :].strip()
        return McpBridgeAdapter(cmd)

    raise ValueError(f"Unsupported adapter spec: {spec!r}")

"""Jev (TypeSafe AI System One decision model) adapter for Playmat Archbench.

Speaks OpenRouter's Decisions API (`POST https://openrouter.ai/api/alpha/decisions`) or any compatible Jev endpoint.
Maps Playmat's zero-leakage game state and legal candidates directly into Jev's `choice` primitive (and optional
`score` board-evaluation primitive), capturing choice probabilities, confidence, token usage, and USD cost.
"""

from __future__ import annotations

import json
import os
import urllib.request
from typing import Any, Dict, List, Optional

from .base import Adapter, compact_state_dict

DEFAULT_JEV_ENDPOINT = "https://openrouter.ai/api/alpha/decisions"
DEFAULT_JEV_MODEL = "typesafe/jev-1.13"


class JevAdapter(Adapter):
    """Adapter for TypeSafe AI's Jev structured decision model (`choice` + optional `score`)."""

    def __init__(
        self,
        model: str = DEFAULT_JEV_MODEL,
        *,
        endpoint: Optional[str] = None,
        api_key: Optional[str] = None,
        with_value: bool = False,
        include_rules: bool = True,
        timeout: float = 30.0,
    ) -> None:
        super().__init__(name=f"jev:{model}")
        self.model = model
        self.endpoint = endpoint or os.environ.get("JEV_ENDPOINT") or DEFAULT_JEV_ENDPOINT
        self.api_key = (
            api_key
            or os.environ.get("JEV_API_KEY")
            or os.environ.get("OPENROUTER_API_KEY")
            or ""
        )
        self.with_value = with_value
        self.include_rules = include_rules
        self.timeout = timeout
        self.mock = model == "mock"

    def build_payload(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        criteria = {c["id"]: c["label"] for c in cands}
        questions: Dict[str, Any] = {
            "action": {
                "type": "choice",
                "instructions": (
                    f"You are playing competitive Magic: The Gathering. {req.get('query_text', '')} "
                    "Choose the highest-win-probability legal play based on the current board, stack, life totals, and cards."
                ),
                "criteria": criteria,
            }
        }
        if self.with_value:
            questions["win_outlook"] = {
                "type": "score",
                "instructions": "Evaluate your current position in this Magic: The Gathering game.",
                "criteria": [
                    "Losing badly",
                    "Slightly behind",
                    "Even position",
                    "Slightly ahead",
                    "Winning decisively",
                ],
            }
        return {
            "model": self.model,
            "state": compact_state_dict(req, include_rules=self.include_rules),
            "questions": questions,
        }

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        if self.mock:
            return self._mock_decide(req, cands)
        if not self.api_key:
            raise RuntimeError("OPENROUTER_API_KEY or JEV_API_KEY is required for JevAdapter")

        payload = self.build_payload(req)
        body = json.dumps(payload).encode("utf-8")
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://arcanite24.github.io/playmat/benchmarks/",
            "X-Title": "Playmat Archbench",
        }
        http_req = urllib.request.Request(self.endpoint, data=body, headers=headers, method="POST")
        with urllib.request.urlopen(http_req, timeout=self.timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        return self.parse_response(data, cands)

    def parse_response(self, data: Dict[str, Any], cands: List[Dict[str, Any]]) -> Dict[str, Any]:
        answers = data.get("answers") or {}
        action = answers.get("action") or {}
        choice_id = action.get("choice")
        id_to_idx = {c["id"]: int(c["index"]) for c in cands}

        probs_map = action.get("probabilities") or {}
        probs: List[float] = [float(probs_map.get(c["id"], 0.0)) for c in cands]
        s = sum(probs)
        if s > 0:
            probs = [p / s for p in probs]

        if choice_id in id_to_idx:
            chosen = id_to_idx[choice_id]
        elif probs and max(probs) > 0:
            chosen = max(range(len(probs)), key=lambda i: probs[i])
        else:
            chosen = 0

        value = None
        if "win_outlook" in answers and isinstance(answers["win_outlook"], dict):
            sc = answers["win_outlook"].get("score")
            if isinstance(sc, (int, float)):
                # score spans 0..4 over 5 criteria -> normalize to 0..1
                value = max(0.0, min(1.0, float(sc) / 4.0))

        usage = data.get("usage") or {}
        return {
            "choice": chosen,
            "probabilities": probs if s > 0 else None,
            "value": value,
            "nodes": 1,
            "input_tokens": int(usage.get("input_tokens", 0) or 0),
            "output_tokens": int(usage.get("output_tokens", 0) or 0),
            "cost_usd": float(usage.get("cost", 0.0) or 0.0),
        }

    def _mock_decide(self, req: Dict[str, Any], cands: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Deterministic offline mock for unit and harness tests (prefers non-pass plays when available)."""
        chosen = 0
        for c in cands:
            lbl = (c.get("label") or "").lower()
            if lbl.startswith("play land:"):
                chosen = int(c["index"])
                break
        else:
            if len(cands) > 1 and req.get("query_kind") == "priority" and (cands[0].get("label") or "") == "Pass priority":
                chosen = 1
        probs = [0.05] * len(cands)
        probs[chosen] = 1.0
        tot = sum(probs)
        return {
            "choice": chosen,
            "probabilities": [round(p / tot, 4) for p in probs],
            "value": 0.52,
            "nodes": 1,
            "input_tokens": 120,
            "output_tokens": 16,
            "cost_usd": 0.00001,
        }

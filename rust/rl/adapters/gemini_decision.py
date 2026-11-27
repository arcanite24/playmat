"""Google Gemini Decision 1 (`gemini-decision-1`) adapter for Playmat Archbench.

Speaks the official Gemini Decisions API (`POST https://generativelanguage.googleapis.com/v1beta/decisions`)
with `x-goog-api-key: $GEMINI_API_KEY`. Evaluates legal MTG choices in a single prefill pass with 0 output tokens
($0.10 / 1M input tokens), returning calibrated choice probabilities, confidence, and optional board-value score.
"""

from __future__ import annotations

import json
import os
import urllib.request
from typing import Any, Dict, List, Optional

from .base import Adapter, format_state_text

DEFAULT_GEMINI_DECISIONS_ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/decisions"
DEFAULT_GEMINI_DECISION_MODEL = "gemini-decision-1"
INPUT_PRICE_PER_1M = 0.10


class GeminiDecisionAdapter(Adapter):
    """Adapter for Google's calibrated Gemini Decision model (`gemini-decision-1`, 0 output tokens)."""

    def __init__(
        self,
        model: str = DEFAULT_GEMINI_DECISION_MODEL,
        *,
        endpoint: Optional[str] = None,
        api_key: Optional[str] = None,
        with_value: bool = False,
        include_rules: bool = True,
        timeout: float = 30.0,
    ) -> None:
        super().__init__(name=f"gemini-decision:{model}")
        self.model = model
        self.endpoint = endpoint or os.environ.get("GEMINI_DECISIONS_ENDPOINT") or DEFAULT_GEMINI_DECISIONS_ENDPOINT
        self.api_key = (
            api_key
            or os.environ.get("GEMINI_API_KEY")
            or os.environ.get("GOOGLE_API_KEY")
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
                    f"You are an expert competitive Magic: The Gathering player. {req.get('query_text', '')} "
                    "Select the single highest-win-probability legal choice based on the board state, stack, life totals, and cards."
                ),
                "criteria": criteria,
            }
        }
        if self.with_value:
            questions["win_outlook"] = {
                "type": "score",
                "instructions": "Evaluate your current win probability in this Magic: The Gathering game.",
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
            "input": format_state_text(req, include_rules=self.include_rules),
            "questions": questions,
        }

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        if self.mock:
            chosen = 1 if len(cands) > 1 and (cands[1].get("label") or "").startswith("Play land:") else 0
            probs = [0.05] * len(cands)
            probs[chosen] = 1.0
            tot = sum(probs)
            return {
                "choice": chosen,
                "probabilities": [round(p / tot, 4) for p in probs],
                "value": 0.55,
                "nodes": 1,
                "input_tokens": 116,
                "output_tokens": 0,
                "cost_usd": 116 * INPUT_PRICE_PER_1M / 1_000_000.0,
            }

        if not self.api_key:
            raise RuntimeError("GEMINI_API_KEY or GOOGLE_API_KEY is required for GeminiDecisionAdapter")

        payload = self.build_payload(req)
        http_req = urllib.request.Request(
            self.endpoint,
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "x-goog-api-key": self.api_key,
                "Content-Type": "application/json",
            },
            method="POST",
        )
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
                value = max(0.0, min(1.0, float(sc) / 4.0))

        usage = data.get("usage") or {}
        in_tok = int(usage.get("total_input_tokens", usage.get("total_tokens", 0)) or 0)
        out_tok = int(usage.get("total_output_tokens", 0) or 0)
        cost = in_tok * INPUT_PRICE_PER_1M / 1_000_000.0
        return {
            "choice": chosen,
            "probabilities": probs if s > 0 else None,
            "value": value,
            "nodes": 1,
            "input_tokens": in_tok,
            "output_tokens": out_tok,
            "cost_usd": cost,
        }

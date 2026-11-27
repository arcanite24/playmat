"""Universal LLM adapter for Playmat Archbench.

Supports OpenRouter (Claude, GPT, Gemini, DeepSeek, Qwen, Llama, GLM, Kimi, Grok), OpenAI, Anthropic, Google Gemini,
DeepSeek, and local OpenAI-compatible servers (Ollama, vLLM, llama.cpp, LM Studio) using Python stdlib HTTP only.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import threading
import time
import urllib.request
from typing import Any, Dict, List, Optional, Tuple

from .base import Adapter, format_state_text

SYSTEM_PROMPT = (
    "You are an expert competitive Magic: The Gathering player piloting a deck in a 1v1 game under strict official rules. "
    "You will be given the current visible game state and a numbered list of legal choices. "
    "Pick the single best legal choice index to maximize your win probability. "
    'Respond with ONLY a single JSON object of the form {"choice": <integer index>, "reason": "<brief 1-sentence reason>"}.'
)

# Approximate fallback pricing ($ per 1M tokens: input, output) when provider doesn't return usage.cost
FALLBACK_PRICING: Dict[str, Tuple[float, float]] = {
    "claude-opus": (15.0, 75.0),
    "claude-sonnet": (3.0, 15.0),
    "claude-haiku": (0.8, 4.0),
    "gpt-5-mini": (0.25, 2.0),
    "gpt-5.4-mini": (0.75, 4.5),
    "gpt-5.2": (1.75, 14.0),
    "gpt-5": (2.5, 10.0),
    "gpt-4.1-mini": (0.40, 1.60),
    "gpt-4.1": (2.0, 8.0),
    "gpt-4o-mini": (0.15, 0.6),
    "o3": (2.0, 8.0),
    "gemini-3.5-flash": (0.50, 3.0),
    "gemini-3.1-pro": (2.0, 12.0),
    "gemini-3-pro": (2.0, 12.0),
    "gemini-2.5-pro": (1.25, 10.0),
    "gemini-2.5-flash": (0.15, 0.6),
    "deepseek-v4": (0.14, 0.42),
    "deepseek": (0.28, 0.42),
    "glm-5": (0.60, 2.0),
    "gemma-4": (0.10, 0.30),
    "gpt-oss-120b": (0.15, 0.60),
}


_VERTEX_LOCK = threading.Lock()
_VERTEX_PROJECT_CACHE: Optional[str] = None
_VERTEX_TOKEN_CACHE: Tuple[str, float] = ("", 0.0)


def _resolve_vertex_project() -> str:
    global _VERTEX_PROJECT_CACHE
    env_pid = os.environ.get("VERTEX_PROJECT_ID") or os.environ.get("GOOGLE_CLOUD_PROJECT")
    if env_pid:
        return env_pid.strip()
    with _VERTEX_LOCK:
        if _VERTEX_PROJECT_CACHE:
            return _VERTEX_PROJECT_CACHE
        raw = subprocess.check_output(["gcloud", "config", "get-value", "project"], text=True).strip()
        if not raw:
            raise RuntimeError("No GCP project configured (set VERTEX_PROJECT_ID or run `gcloud config set project <ID>`)")
        if raw.isdigit():
            raw = subprocess.check_output(
                ["gcloud", "projects", "describe", raw, "--format=value(projectId)"], text=True
            ).strip()
        _VERTEX_PROJECT_CACHE = raw
        return raw


def _resolve_vertex_token() -> str:
    global _VERTEX_TOKEN_CACHE
    env_tok = os.environ.get("VERTEX_ACCESS_TOKEN")
    if env_tok:
        return env_tok.strip()
    with _VERTEX_LOCK:
        tok, fetched_at = _VERTEX_TOKEN_CACHE
        # gcloud access tokens expire after 60 min; refresh proactively every 45 min
        if tok and (time.time() - fetched_at) < 2700:
            return tok
        tok = subprocess.check_output(["gcloud", "auth", "print-access-token"], text=True).strip()
        if not tok:
            raise RuntimeError("Could not obtain GCP access token (`gcloud auth print-access-token`)")
        _VERTEX_TOKEN_CACHE = (tok, time.time())
        return tok


def _estimate_cost(model: str, in_tok: int, out_tok: int) -> float:
    low = model.lower()
    for prefix, (pin, pout) in FALLBACK_PRICING.items():
        if prefix in low:
            return (in_tok * pin + out_tok * pout) / 1_000_000.0
    return 0.0


def _extract_choice_json(text: str, k: int) -> Optional[Tuple[int, str]]:
    """Extracts {"choice": N, "reason": ...} from raw or markdown-fenced LLM output."""
    s = (text or "").strip()
    if not s:
        return None
    # Try direct JSON or fenced JSON block
    candidates = [s]
    for m in re.finditer(r"\{[^{}]*\"choice\"[^{}]*\}", s, flags=re.DOTALL):
        candidates.insert(0, m.group(0))
    for blob in candidates:
        try:
            obj = json.loads(blob)
            if isinstance(obj, dict) and "choice" in obj:
                raw = obj["choice"]
                if isinstance(raw, str):
                    raw = raw.strip().lstrip("cC")
                idx = int(raw)
                if 0 <= idx < k:
                    return idx, str(obj.get("reason", ""))[:200]
        except Exception:
            continue
    # Last-resort regex for "choice": N or standalone leading index
    m = re.search(r"\"?choice\"?\s*[:=]\s*\"?c?(\d+)\"?", s, flags=re.IGNORECASE)
    if m:
        idx = int(m.group(1))
        if 0 <= idx < k:
            return idx, ""
    return None


class LlmAdapter(Adapter):
    """Multi-provider LLM adapter for Playmat's playmat-ext-v1 decision loop."""

    def __init__(
        self,
        provider: str,
        model: str,
        *,
        base_url: Optional[str] = None,
        api_key: Optional[str] = None,
        temperature: float = 0.0,
        max_tokens: int = 256,
        reasoning_effort: Optional[str] = None,
        include_rules: bool = True,
        timeout: float = 60.0,
    ) -> None:
        super().__init__(name=f"llm:{provider}/{model}")
        self.provider = provider.lower()
        self.model = model
        self.base_url = base_url
        self.api_key = api_key or self._resolve_key(self.provider)
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.reasoning_effort = reasoning_effort
        self.include_rules = include_rules
        self.timeout = timeout

    @staticmethod
    def _resolve_key(provider: str) -> str:
        env_map = {
            "openrouter": "OPENROUTER_API_KEY",
            "openai": "OPENAI_API_KEY",
            "anthropic": "ANTHROPIC_API_KEY",
            "gemini": "GEMINI_API_KEY",
            "deepseek": "DEEPSEEK_API_KEY",
            "groq": "GROQ_API_KEY",
            "together": "TOGETHER_API_KEY",
            "fireworks": "FIREWORKS_API_KEY",
            "vllm": "OPENAI_API_KEY",
            "ollama": "OLLAMA_API_KEY",
            "lmstudio": "OPENAI_API_KEY",
            "openai-compat": "OPENAI_API_KEY",
        }
        var = env_map.get(provider, "OPENAI_API_KEY")
        return os.environ.get(var, "")

    def _chat_endpoint(self) -> str:
        if self.base_url:
            u = self.base_url.rstrip("/")
            return u if u.endswith("/chat/completions") else f"{u}/chat/completions"
        if self.provider in ("vertex-maas", "vertex"):
            pid = _resolve_vertex_project()
            loc = os.environ.get("VERTEX_LOCATION", "global")
            host = "aiplatform.googleapis.com" if loc == "global" else f"{loc}-aiplatform.googleapis.com"
            return f"https://{host}/v1/projects/{pid}/locations/{loc}/endpoints/openapi/chat/completions"
        defaults = {
            "openrouter": "https://openrouter.ai/api/v1/chat/completions",
            "openai": "https://api.openai.com/v1/chat/completions",
            "deepseek": "https://api.deepseek.com/v1/chat/completions",
            "groq": "https://api.groq.com/openai/v1/chat/completions",
            "together": "https://api.together.xyz/v1/chat/completions",
            "fireworks": "https://api.fireworks.ai/inference/v1/chat/completions",
            "ollama": os.environ.get("OLLAMA_HOST", "http://127.0.0.1:11434").rstrip("/") + "/v1/chat/completions",
            "lmstudio": "http://127.0.0.1:1234/v1/chat/completions",
            "vllm": os.environ.get("OPENAI_BASE_URL", "http://127.0.0.1:8000/v1").rstrip("/") + "/chat/completions",
            "openai-compat": os.environ.get("OPENAI_BASE_URL", "http://127.0.0.1:8000/v1").rstrip("/") + "/chat/completions",
        }
        return defaults[self.provider]

    def _call_openai_compat(self, prompt: str) -> Tuple[str, int, int, float]:
        url = self._chat_endpoint()
        is_vertex = self.provider in ("vertex-maas", "vertex")
        local = self.provider in ("ollama", "lmstudio", "vllm", "openai-compat")
        token = _resolve_vertex_token() if is_vertex else self.api_key
        if not local and not token:
            raise RuntimeError(f"Missing API key/token for provider '{self.provider}'")
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        if self.provider == "openrouter":
            headers["HTTP-Referer"] = "https://arcanite24.github.io/playmat/benchmarks/"
            headers["X-Title"] = "Playmat Archbench"

        is_openai_reasoning = self.provider == "openai" and (
            self.model.startswith("gpt-5") or self.model.startswith(("o1", "o3", "o4"))
        )
        body_obj: Dict[str, Any] = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt},
            ],
        }
        if is_openai_reasoning:
            # Reasoning models use max_completion_tokens (allow headroom for reasoning tokens) and default temperature
            body_obj["max_completion_tokens"] = max(self.max_tokens, 1024)
            body_obj["reasoning_effort"] = self.reasoning_effort or "low"
        elif is_vertex:
            # Vertex MaaS partner models (GLM, DeepSeek, Gemini, GPT-OSS, Gemma) may emit thinking tokens before JSON
            body_obj["temperature"] = self.temperature
            body_obj["max_tokens"] = max(self.max_tokens, 1536)
            if self.reasoning_effort:
                body_obj["reasoning_effort"] = self.reasoning_effort
        else:
            body_obj["temperature"] = self.temperature
            body_obj["max_tokens"] = self.max_tokens
            if self.reasoning_effort:
                body_obj["reasoning_effort"] = self.reasoning_effort
        if self.provider in ("openai", "openrouter", "deepseek", "ollama"):
            body_obj["response_format"] = {"type": "json_object"}

        req = urllib.request.Request(url, data=json.dumps(body_obj).encode("utf-8"), headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=self.timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        text = ((data.get("choices") or [{}])[0].get("message") or {}).get("content") or ""
        usage = data.get("usage") or {}
        in_tok = int(usage.get("prompt_tokens", 0) or 0)
        out_tok = int(usage.get("completion_tokens", 0) or 0)
        cost = float(usage.get("cost", 0.0) or 0.0)
        if cost == 0.0 and not local:
            cost = _estimate_cost(self.model, in_tok, out_tok)
        return text, in_tok, out_tok, cost

    def _call_anthropic(self, prompt: str) -> Tuple[str, int, int, float]:
        if not self.api_key:
            raise RuntimeError("Missing ANTHROPIC_API_KEY")
        url = (self.base_url or "https://api.anthropic.com/v1/messages").rstrip("/")
        headers = {
            "x-api-key": self.api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json",
        }
        body_obj = {
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "system": SYSTEM_PROMPT,
            "messages": [{"role": "user", "content": prompt}],
        }
        req = urllib.request.Request(url, data=json.dumps(body_obj).encode("utf-8"), headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=self.timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        blocks = data.get("content") or []
        text = "".join(b.get("text", "") for b in blocks if b.get("type") == "text")
        usage = data.get("usage") or {}
        in_tok = int(usage.get("input_tokens", 0) or 0)
        out_tok = int(usage.get("output_tokens", 0) or 0)
        return text, in_tok, out_tok, _estimate_cost(self.model, in_tok, out_tok)

    def _call_gemini(self, prompt: str) -> Tuple[str, int, int, float]:
        if not self.api_key:
            raise RuntimeError("Missing GEMINI_API_KEY")
        base = (self.base_url or "https://generativelanguage.googleapis.com/v1beta").rstrip("/")
        url = f"{base}/models/{self.model}:generateContent?key={self.api_key}"
        body_obj = {
            "systemInstruction": {"parts": [{"text": SYSTEM_PROMPT}]},
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_tokens,
                "responseMimeType": "application/json",
            },
        }
        req = urllib.request.Request(
            url,
            data=json.dumps(body_obj).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=self.timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        cands = data.get("candidates") or []
        parts = ((cands[0].get("content") or {}).get("parts") or []) if cands else []
        text = "".join(p.get("text", "") for p in parts)
        meta = data.get("usageMetadata") or {}
        in_tok = int(meta.get("promptTokenCount", 0) or 0)
        out_tok = int(meta.get("candidatesTokenCount", 0) or 0)
        return text, in_tok, out_tok, _estimate_cost(self.model, in_tok, out_tok)

    def _call_provider(self, prompt: str) -> Tuple[str, int, int, float]:
        if self.provider == "mock":
            return '{"choice": 0, "reason": "mock quick pick"}', 80, 12, 0.0
        if self.provider == "anthropic":
            return self._call_anthropic(prompt)
        if self.provider == "gemini":
            return self._call_gemini(prompt)
        return self._call_openai_compat(prompt)

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        prompt = format_state_text(req, include_rules=self.include_rules)
        text, in_tok, out_tok, cost = self._call_provider(prompt)
        parsed = _extract_choice_json(text, len(cands))
        if parsed is None:
            # One concise retry asking strictly for the integer choice
            retry_prompt = (
                prompt
                + f"\n\nYour previous reply could not be parsed. Reply ONLY with JSON {{\"choice\": N}} where 0 <= N < {len(cands)}."
            )
            text2, in2, out2, cost2 = self._call_provider(retry_prompt)
            in_tok += in2
            out_tok += out2
            cost += cost2
            parsed = _extract_choice_json(text2, len(cands))

        if parsed is None:
            return {
                "choice": 0,
                "nodes": 1,
                "fallback": True,
                "input_tokens": in_tok,
                "output_tokens": out_tok,
                "cost_usd": cost,
            }
        idx, reason = parsed
        return {
            "choice": idx,
            "nodes": 1,
            "reason": reason,
            "input_tokens": in_tok,
            "output_tokens": out_tok,
            "cost_usd": cost,
        }

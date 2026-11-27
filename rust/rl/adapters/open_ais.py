"""Adapters for open-source MTG AIs, heuristic baselines, ONNX neural policies, Python plugins, HTTP agents,
and mage-bench-style MCP tool bridges.
"""

from __future__ import annotations

import importlib
import json
import math
import subprocess
import urllib.request
from typing import Any, Callable, Dict, List, Optional

from .base import Adapter, format_state_text


def _threat_score(card: Dict[str, Any]) -> float:
    """Forge-style per-card static threat score for creatures, planeswalkers, and permanents."""
    types = set(card.get("types") or [])
    mv = float(card.get("mana_value", 0) or 0)
    score = mv * 1.2
    if "Creature" in types:
        p = float(card.get("power", 0) or 0)
        t = float(card.get("toughness", 0) or 0)
        score += p * 2.2 + t * 1.1
        kws = set(card.get("keywords") or [])
        for kw, bonus in (
            ("Flying", 1.8),
            ("Double Strike", 2.5),
            ("Lifelink", 1.5),
            ("Deathtouch", 1.6),
            ("First Strike", 1.2),
            ("Trample", 1.2),
            ("Hexproof", 1.4),
            ("Indestructible", 2.4),
            ("Menace", 1.0),
        ):
            if kw in kws:
                score += bonus
        if card.get("damage"):
            score -= float(card["damage"]) * 0.5
    if "Planeswalker" in types:
        score += 6.0
    if "Land" in types:
        score = 0.5
    return score


class ForgeHeuristicAdapter(Adapter):
    """Forge-inspired `canPlayAI()` greedy single-card & combat-loop heuristic baseline over Playmat's engine.

    Useful as an apples-to-apples rule-based reference on Archbench suites without needing Forge's separate engine.
    """

    def __init__(self, style: str = "forge-heuristic") -> None:
        super().__init__(name=f"open:{style}")
        self.style = style

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        qkind = req.get("query_kind") or ""
        st = req.get("state") or {}
        you = st.get("you") or {}
        opp = st.get("opponent") or {}
        opp_field = {c["id"]: c for c in (opp.get("battlefield") or []) if "id" in c}
        you_field = {c["id"]: c for c in (you.get("battlefield") or []) if "id" in c}
        you_hand = {c["id"]: c for c in (you.get("hand") or []) if "id" in c}

        if qkind == "priority":
            # 1. Always play a land first in main phase
            for c in cands:
                if (c.get("label") or "").startswith("Play land:"):
                    return {"choice": int(c["index"]), "nodes": 1}
            # 2. On own main phase, play highest-value threat / spell on curve
            if st.get("active_player") == "you" and st.get("step") in ("PRECOMBAT_MAIN", "POSTCOMBAT_MAIN"):
                best_idx = 0
                best_score = -1.0
                for c in cands:
                    if c.get("kind") == "pass_or_no":
                        continue
                    oid = c.get("obj_id")
                    card = you_hand.get(oid) or you_field.get(oid)
                    if not card:
                        continue
                    types = set(card.get("types") or [])
                    mv = float(card.get("mana_value", 0) or 0)
                    sc = mv * 2.0 + _threat_score(card)
                    if self.style == "greedy-aggro" and "Creature" in types:
                        sc += 3.0
                    elif self.style == "greedy-control" and "Instant" in types:
                        sc -= 2.5  # prefer holding instants
                    if sc > best_score:
                        best_score = sc
                        best_idx = int(c["index"])
                return {"choice": best_idx, "nodes": 1}
            # 3. On opponent's turn, cast instants if stack is non-empty or end step
            if st.get("active_player") == "opponent" and (st.get("stack") or st.get("step") == "END_TURN"):
                for c in cands:
                    if c.get("kind") != "pass_or_no" and (c.get("label") or "").startswith("Cast "):
                        return {"choice": int(c["index"]), "nodes": 1}
            # Default: pass
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    return {"choice": int(c["index"]), "nodes": 1}
            return {"choice": 0, "nodes": 1}

        if qkind == "target":
            # Prefer opponent's highest-threat permanent or opponent face if low life
            best_idx = 0
            best_score = -1e9
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    continue
                if c.get("kind") == "player":
                    lbl = (c.get("label") or "").lower()
                    sc = 8.0 if "opponent" in lbl else -20.0
                    if "opponent" in lbl and int(opp.get("life", 20)) <= 6:
                        sc += 12.0
                else:
                    oid = c.get("obj_id")
                    if oid in opp_field:
                        sc = 5.0 + _threat_score(opp_field[oid])
                    elif oid in you_field:
                        sc = -5.0 - _threat_score(you_field[oid]) * 0.2
                    else:
                        sc = 0.0
                if sc > best_score:
                    best_score = sc
                    best_idx = int(c["index"])
            return {"choice": best_idx, "nodes": 1}

        if qkind == "attackers":
            # Forge-style single-creature attack loop: attack if unblockable, stronger than best blocker, or aggro
            opp_untapped = [
                c for c in opp_field.values() if "Creature" in (c.get("types") or []) and not c.get("tapped")
            ]
            max_opp_p = max((int(c.get("power", 0) or 0) for c in opp_untapped), default=0)
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    continue
                oid = c.get("obj_id")
                me = you_field.get(oid)
                if not me:
                    continue
                p = int(me.get("power", 0) or 0)
                t = int(me.get("toughness", 0) or 0)
                kws = set(me.get("keywords") or [])
                if p <= 0:
                    continue
                if (
                    not opp_untapped
                    or "Flying" in kws
                    or "Indestructible" in kws
                    or t > max_opp_p
                    or self.style == "greedy-aggro"
                    or int(opp.get("life", 20)) <= p
                ):
                    return {"choice": int(c["index"]), "nodes": 1}
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    return {"choice": int(c["index"]), "nodes": 1}
            return {"choice": 0, "nodes": 1}

        if qkind == "blockers":
            # Block if incoming damage threatens lethal or a creature can trade favorably
            attackers = (st.get("combat") or {}).get("attackers") or []
            incoming = sum(int(a.get("power", 0) or 0) for a in attackers if not a.get("blocked"))
            lethal = incoming >= int(you.get("life", 20))
            if lethal:
                for c in cands:
                    if c.get("kind") != "pass_or_no":
                        return {"choice": int(c["index"]), "nodes": 1}
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    continue
                me = you_field.get(c.get("obj_id"))
                if not me:
                    continue
                p = int(me.get("power", 0) or 0)
                t = int(me.get("toughness", 0) or 0)
                kws = set(me.get("keywords") or [])
                if any(
                    ("Deathtouch" in kws and p > 0)
                    or (p >= int(a.get("toughness", 0) or 0) and t > int(a.get("power", 0) or 0))
                    for a in attackers
                ):
                    return {"choice": int(c["index"]), "nodes": 1}
            for c in cands:
                if c.get("kind") == "pass_or_no":
                    return {"choice": int(c["index"]), "nodes": 1}
            return {"choice": 0, "nodes": 1}

        return {"choice": 0, "nodes": 1}


class OnnxPolicyAdapter(Adapter):
    """Runs any external ONNX policy/value model through `onnxruntime` on Playmat's numeric `obs` tensors.

    Supported input tensor names (matched automatically from the model's inputs):
      - `glob`: float32 [1, GLOB_F]
      - `ents`: float32 [1, N_ENT, ENT_F]
      - `ent_ids`: int64 [1, N_ENT]
      - `acts`: float32 [1, N_ACT, ACT_F]
      - `act_ids`: int64 [1, N_ACT]
    Expected first output: candidate logits `[1, N_ACT]` or `[N_ACT]`; optional second output: win probability scalar.
    """

    needs_obs: bool = True

    def __init__(self, model_path: str) -> None:
        super().__init__(name=f"onnx:{model_path}")
        try:
            import numpy as np  # type: ignore
            import onnxruntime as ort  # type: ignore
        except ImportError as exc:
            raise RuntimeError(
                "OnnxPolicyAdapter requires `onnxruntime` and `numpy` (pip install onnxruntime numpy)"
            ) from exc
        self._np = np
        self.session = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
        self.input_names = [inp.name for inp in self.session.get_inputs()]

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        np = self._np
        obs = req.get("obs")
        cands = req.get("candidates") or []
        if not obs:
            raise RuntimeError("OnnxPolicyAdapter requires ?obs=1 on the external endpoint")
        n_ent = max(1, len(obs.get("ent_ids") or []))
        n_act = max(1, len(cands))
        glob = np.asarray(obs["glob"], dtype=np.float32).reshape(1, -1)
        ents_raw = np.asarray(obs.get("ents") or [], dtype=np.float32)
        ents = ents_raw.reshape(1, n_ent, -1) if ents_raw.size else np.zeros((1, 1, 49), dtype=np.float32)
        ent_ids = np.asarray(obs.get("ent_ids") or [0], dtype=np.int64).reshape(1, -1)
        acts = np.asarray(obs["acts"], dtype=np.float32).reshape(1, n_act, -1)
        act_ids = np.asarray(obs["act_ids"], dtype=np.int64).reshape(1, -1)

        available = {
            "glob": glob,
            "ents": ents,
            "ent_ids": ent_ids,
            "acts": acts,
            "act_ids": act_ids,
        }
        feeds = {name: available[name] for name in self.input_names if name in available}
        outputs = self.session.run(None, feeds)
        logits = np.asarray(outputs[0], dtype=np.float64).reshape(-1)[: len(cands)]
        m = float(np.max(logits))
        exps = [math.exp(float(x) - m) for x in logits]
        s = sum(exps) or 1.0
        probs = [e / s for e in exps]
        best = max(range(len(probs)), key=lambda i: probs[i])
        val = None
        if len(outputs) > 1:
            v = float(np.asarray(outputs[1]).reshape(-1)[0])
            if math.isfinite(v):
                val = 1.0 / (1.0 + math.exp(-v)) if (v < 0.0 or v > 1.0) else v
        return {"choice": best, "probabilities": probs, "value": val, "nodes": 1}


class PythonPluginAdapter(Adapter):
    """Loads any user Python callable or class (`module.sub:ClassOrFunc`) as a Playmat adapter.

    Enables direct integration with MageZero PyTorch policies, MTG-Causal-RL agents, or custom research scripts.
    """

    def __init__(self, target: str, *, needs_obs: bool = False) -> None:
        super().__init__(name=f"python:{target}")
        self.needs_obs = needs_obs
        if ":" not in target:
            raise ValueError(f"python adapter spec must be 'module:attr', got {target!r}")
        mod_name, attr_name = target.split(":", 1)
        mod = importlib.import_module(mod_name)
        obj = getattr(mod, attr_name)
        if isinstance(obj, type):
            inst = obj()
            self._fn: Callable[[Dict[str, Any]], Any] = inst.decide if hasattr(inst, "decide") else inst
        else:
            self._fn = obj

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        res = self._fn(req)
        if isinstance(res, int):
            return {"choice": res, "nodes": 1}
        if isinstance(res, dict):
            return res
        raise ValueError(f"PythonPluginAdapter callable returned unsupported type: {type(res)}")


class HttpAgentAdapter(Adapter):
    """Forwards `playmat-ext-v1` requests via HTTP POST to any external agent server (Java/Kotlin/Go/Node/Python)."""

    def __init__(self, url: str, *, timeout: float = 30.0, needs_obs: bool = False) -> None:
        super().__init__(name=f"http:{url}")
        self.url = url
        self.timeout = timeout
        self.needs_obs = needs_obs

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        body = json.dumps(req).encode("utf-8")
        http_req = urllib.request.Request(
            self.url,
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(http_req, timeout=self.timeout) as resp:
            return json.loads(resp.read().decode("utf-8"))


class McpBridgeAdapter(Adapter):
    """Bridges a mage-bench-style MCP / JSON-RPC 2.0 stdio agent into Playmat's Archbench decision loop.

    Starts the configured MCP agent command once and invokes `tools/call` with `name="choose_action"` passing
    the formatted MTG board state and numbered legal candidates, compatible with MCP MTG harnesses.
    """

    def __init__(self, command: str) -> None:
        super().__init__(name=f"mcp:{command}")
        self.command = command
        self._proc: Optional[subprocess.Popen[str]] = None
        self._next_id = 1

    def _ensure_proc(self) -> subprocess.Popen[str]:
        if self._proc is None or self._proc.poll() is not None:
            self._proc = subprocess.Popen(
                self.command,
                shell=True,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                text=True,
                bufsize=1,
            )
            self._rpc("initialize", {"protocolVersion": "2024-11-05", "clientInfo": {"name": "playmat-archbench"}})
        return self._proc

    def _rpc(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        proc = self._proc
        assert proc is not None and proc.stdin is not None and proc.stdout is not None
        req_id = self._next_id
        self._next_id += 1
        msg = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
        proc.stdin.write(json.dumps(msg) + "\n")
        proc.stdin.flush()
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("MCP subprocess closed stdout")
        return json.loads(line)

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        self._ensure_proc()
        res = self._rpc(
            "tools/call",
            {
                "name": "choose_action",
                "arguments": {
                    "board_text": format_state_text(req, include_rules=True),
                    "state": req.get("state"),
                    "query_kind": req.get("query_kind"),
                    "query_text": req.get("query_text"),
                    "candidates": req.get("candidates"),
                },
            },
        )
        result = res.get("result") or {}
        if "choice" in result:
            return result
        # Standard MCP content array fallback
        for item in result.get("content") or []:
            if item.get("type") == "text":
                parsed = json.loads(item["text"])
                if isinstance(parsed, dict) and "choice" in parsed:
                    return parsed
        return {"choice": 0, "nodes": 1, "fallback": True}

    def close(self) -> None:
        if self._proc is not None:
            try:
                if self._proc.stdin:
                    self._proc.stdin.close()
                if self._proc.stdout:
                    self._proc.stdout.close()
                self._proc.terminate()
                self._proc.wait(timeout=3)
            except Exception:
                self._proc.kill()
            self._proc = None

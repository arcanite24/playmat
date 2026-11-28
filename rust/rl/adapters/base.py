"""Base classes, card rules text lookup, telemetry, and TCP/stdio servers for Playmat external AI adapters.

Protocol (`playmat-ext-v1`): line-delimited JSON (NDJSON) over TCP or stdin/stdout.
Each request contains a zero-leakage `state`, `query_kind`, `query_text`, `candidates`, and optional numeric `obs`.
Each response returns `{"choice": int, "probabilities": [...], "value": float, "nodes": int, "reason": str}`.
"""

from __future__ import annotations

import gzip
import json
import os
import socketserver
import sys
import threading
import time
from typing import Any, Dict, List, Optional

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
VIEWS_JSON = os.path.join(REPO, "rust", "crates", "playmat-engine", "cards", "views.json.gz")


def load_dotenv() -> None:
    """Loads missing env vars from <repo>/.env and ~/.env without overwriting explicit environment values."""
    for path in (os.path.join(REPO, ".env"), os.path.expanduser("~/.env")):
        if not os.path.exists(path):
            continue
        try:
            with open(path, "r", encoding="utf-8", errors="ignore") as f:
                for raw in f:
                    s = raw.strip()
                    if not s or s.startswith("#") or "=" not in s:
                        continue
                    k, v = s.split("=", 1)
                    k = k.strip()
                    v = v.strip().strip("\"'")
                    if k and v and k not in os.environ:
                        os.environ[k] = v
        except Exception:
            pass


load_dotenv()


class CardTextStore:
    """Lazily loads official card rules text from Playmat's views.json for LLM / Jev prompt enrichment."""

    _instance: Optional["CardTextStore"] = None
    _lock = threading.Lock()

    def __init__(self, path: str = VIEWS_JSON) -> None:
        self._rules: Dict[str, str] = {}
        if os.path.exists(path):
            try:
                opener = gzip.open if path.endswith(".gz") else open
                with opener(path, "rt", encoding="utf-8") as f:
                    raw = json.load(f)
                for name, view in raw.items():
                    rules = view.get("rules") or []
                    if rules:
                        self._rules[name] = " ".join(str(r).strip() for r in rules if r)
            except Exception:
                pass

    @classmethod
    def shared(cls) -> "CardTextStore":
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    def rules_of(self, card_name: str) -> str:
        return self._rules.get(card_name, "")


class AdapterTelemetry:
    """Thread-safe latency, token, API cost, and fallback tracker for any external adapter."""

    def __init__(self, name: str) -> None:
        self.name = name
        self._lock = threading.Lock()
        self.decisions = 0
        self.fallbacks = 0
        self.errors = 0
        self.input_tokens = 0
        self.output_tokens = 0
        self.cost_usd = 0.0
        self.latencies_ms: List[float] = []

    def record(
        self,
        latency_ms: float,
        *,
        fallback: bool = False,
        error: bool = False,
        input_tokens: int = 0,
        output_tokens: int = 0,
        cost_usd: float = 0.0,
    ) -> None:
        with self._lock:
            self.decisions += 1
            if fallback:
                self.fallbacks += 1
            if error:
                self.errors += 1
            self.input_tokens += int(input_tokens or 0)
            self.output_tokens += int(output_tokens or 0)
            self.cost_usd += float(cost_usd or 0.0)
            self.latencies_ms.append(float(latency_ms))

    def summary(self) -> Dict[str, Any]:
        with self._lock:
            lats = sorted(self.latencies_ms)
            n = len(lats)
            avg_ms = (sum(lats) / n) if n else 0.0
            p50_ms = lats[n // 2] if n else 0.0
            p99_ms = lats[min(n - 1, int(0.99 * n))] if n else 0.0
            worst_ms = lats[-1] if n else 0.0
            return {
                "adapter": self.name,
                "decisions": self.decisions,
                "fallbacks": self.fallbacks,
                "fallback_rate": round(self.fallbacks / max(self.decisions, 1), 4),
                "errors": self.errors,
                "input_tokens": self.input_tokens,
                "output_tokens": self.output_tokens,
                "cost_usd": round(self.cost_usd, 6),
                "avg_latency_ms": round(avg_ms, 2),
                "p50_latency_ms": round(p50_ms, 2),
                "p99_latency_ms": round(p99_ms, 2),
                "worst_latency_ms": round(worst_ms, 2),
            }


def _fmt_card(c: Dict[str, Any], rules_store: Optional[CardTextStore], include_rules: bool = True) -> str:
    parts = [c.get("name", "?")]
    cost = c.get("mana_cost") or ""
    if cost:
        parts.append(cost)
    if "power" in c and "toughness" in c:
        parts.append(f"{c['power']}/{c['toughness']}")
    flags = []
    if c.get("tapped"):
        flags.append("tapped")
    if c.get("summoning_sick"):
        flags.append("sick")
    if c.get("attacking"):
        flags.append("attacking")
    if c.get("blocking_ids"):
        flags.append(f"blocking:{c['blocking_ids']}")
    if c.get("damage"):
        flags.append(f"dmg:{c['damage']}")
    if c.get("counters"):
        flags.append("counters:" + ",".join(f"{k}={v}" for k, v in c["counters"]))
    kws = c.get("keywords") or []
    if kws:
        flags.extend(kws)
    if flags:
        parts.append("(" + ", ".join(flags) + ")")
    if "id" in c:
        parts.append(f"[id:{c['id']}]")
    base = " ".join(parts)
    if include_rules and rules_store is not None:
        r = rules_store.rules_of(c.get("name", ""))
        if r:
            if len(r) > 180:
                r = r[:177] + "..."
            base += f" -- {r}"
    return base


def _fmt_mana(m: Dict[str, Any]) -> str:
    if not m:
        return "empty"
    bits = [f"{k}:{v}" for k in ("W", "U", "B", "R", "G", "C") if m.get(k)]
    return " ".join(bits) if bits else "empty"


def format_state_text(req: Dict[str, Any], *, include_rules: bool = True) -> str:
    """Formats a zero-leakage playmat-ext-v1 request into compact, readable MTG board text."""
    st = req.get("state") or {}
    you = st.get("you") or {}
    opp = st.get("opponent") or {}
    rules = CardTextStore.shared() if include_rules else None

    lines = [
        f"Turn {st.get('turn')} | Step: {st.get('step')} | Active: {st.get('active_player')}",
        f"You: {you.get('life')} life | Hand: {len(you.get('hand') or [])} | Library: {you.get('library_count')} | Lands played: {you.get('lands_played')} | Mana pool: {_fmt_mana(you.get('mana_pool') or {})}",
        f"Opponent: {opp.get('life')} life | Hand: {opp.get('hand_count')} hidden | Library: {opp.get('library_count')} | Mana pool: {_fmt_mana(opp.get('mana_pool') or {})}",
    ]

    def add_zone(title: str, cards: List[Dict[str, Any]], with_rules: bool) -> None:
        if not cards:
            return
        lines.append(f"{title}:")
        for c in cards:
            lines.append("  - " + _fmt_card(c, rules, include_rules=with_rules))

    add_zone("Your Hand", you.get("hand") or [], with_rules=include_rules)
    add_zone("Your Battlefield", you.get("battlefield") or [], with_rules=include_rules)
    add_zone("Opponent Battlefield", opp.get("battlefield") or [], with_rules=include_rules)

    stack = st.get("stack") or []
    if stack:
        lines.append("Stack (top first):")
        for item in stack:
            tgts = item.get("targets") or []
            t_str = ""
            if tgts:
                t_str = " targeting " + ", ".join(
                    t.get("who") if t.get("kind") == "player" else f"{t.get('name')}[id:{t.get('id')}]"
                    for t in tgts
                )
            kind = "ability of " if item.get("is_ability") else ""
            lines.append(f"  - {item.get('controller')}'s {kind}{item.get('name')}{t_str}")

    add_zone("Your Graveyard", (you.get("graveyard") or [])[-8:], with_rules=False)
    add_zone("Opponent Graveyard", (opp.get("graveyard") or [])[-8:], with_rules=False)

    lines.append(f"Question ({req.get('query_kind')}): {req.get('query_text')}")
    lines.append("Legal Choices:")
    for c in req.get("candidates") or []:
        lines.append(f"  [{c.get('index')}] {c.get('label')}")
    return "\n".join(lines)


def compact_state_dict(req: Dict[str, Any], *, include_rules: bool = True) -> Dict[str, Any]:
    """Structured compact state dictionary suited for decision models like Jev."""
    st = req.get("state") or {}
    you = st.get("you") or {}
    opp = st.get("opponent") or {}
    rules = CardTextStore.shared() if include_rules else None

    def enrich(cards: List[Dict[str, Any]], with_rules: bool) -> List[str]:
        return [_fmt_card(c, rules, include_rules=with_rules) for c in cards]

    return {
        "turn": st.get("turn"),
        "step": st.get("step"),
        "active_player": st.get("active_player"),
        "query_kind": req.get("query_kind"),
        "you": {
            "life": you.get("life"),
            "library_count": you.get("library_count"),
            "lands_played": you.get("lands_played"),
            "mana_pool": _fmt_mana(you.get("mana_pool") or {}),
            "hand": enrich(you.get("hand") or [], with_rules=include_rules),
            "battlefield": enrich(you.get("battlefield") or [], with_rules=include_rules),
            "graveyard_recent": enrich((you.get("graveyard") or [])[-6:], with_rules=False),
        },
        "opponent": {
            "life": opp.get("life"),
            "hand_count": opp.get("hand_count"),
            "library_count": opp.get("library_count"),
            "battlefield": enrich(opp.get("battlefield") or [], with_rules=include_rules),
            "graveyard_recent": enrich((opp.get("graveyard") or [])[-6:], with_rules=False),
        },
        "stack": st.get("stack") or [],
        "combat": st.get("combat") or {},
    }


class Adapter:
    """Base interface implemented by every Playmat external AI adapter."""

    needs_obs: bool = False

    def __init__(self, name: str) -> None:
        self.name = name
        self.telemetry = AdapterTelemetry(name)

    def decide(self, req: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError

    def handle_request(self, req: Dict[str, Any]) -> Dict[str, Any]:
        cands = req.get("candidates") or []
        if not cands:
            return {"choice": 0, "nodes": 0}
        t0 = time.perf_counter()
        try:
            resp = self.decide(req)
            dt_ms = (time.perf_counter() - t0) * 1000.0
            choice = resp.get("choice", 0)
            if isinstance(choice, str):
                s = choice.strip()
                if s.lower().startswith("c"):
                    s = s[1:]
                choice = int(s)
            if not isinstance(choice, int) or choice < 0 or choice >= len(cands):
                self.telemetry.record(dt_ms, fallback=True, error=True)
                return {"choice": 0, "nodes": 1, "fallback": True}
            self.telemetry.record(
                dt_ms,
                fallback=bool(resp.get("fallback", False)),
                error=bool(resp.get("error", False)),
                input_tokens=int(resp.get("input_tokens", 0) or 0),
                output_tokens=int(resp.get("output_tokens", 0) or 0),
                cost_usd=float(resp.get("cost_usd", 0.0) or 0.0),
            )
            out: Dict[str, Any] = {"choice": choice, "nodes": int(resp.get("nodes", 1))}
            if "probabilities" in resp and isinstance(resp["probabilities"], list):
                out["probabilities"] = resp["probabilities"]
            if "value" in resp and resp["value"] is not None:
                out["value"] = float(resp["value"])
            if "reason" in resp and resp["reason"]:
                out["reason"] = str(resp["reason"])
            return out
        except Exception as exc:
            dt_ms = (time.perf_counter() - t0) * 1000.0
            self.telemetry.record(dt_ms, fallback=True, error=True)
            if os.environ.get("PLAYMAT_EXT_DEBUG") in ("1", "true"):
                print(f"[{self.name}] error: {exc}", file=sys.stderr, flush=True)
            return {"choice": 0, "nodes": 1, "fallback": True, "error": str(exc)}


class _ThreadingTcpServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


def start_tcp_server(adapter: Adapter, host: str = "127.0.0.1", port: int = 0) -> tuple[_ThreadingTcpServer, threading.Thread, int]:
    """Starts a multi-threaded NDJSON TCP server for `adapter` in a background daemon thread."""

    class Handler(socketserver.StreamRequestHandler):
        def handle(self) -> None:
            for raw in self.rfile:
                line = raw.decode("utf-8", errors="replace").strip()
                if not line:
                    continue
                try:
                    req = json.loads(line)
                except Exception:
                    resp = {"choice": 0, "nodes": 0, "error": "invalid json"}
                else:
                    if req.get("kind") == "telemetry":
                        resp = adapter.telemetry.summary()
                    else:
                        resp = adapter.handle_request(req)
                self.wfile.write((json.dumps(resp) + "\n").encode("utf-8"))
                self.wfile.flush()

    srv = _ThreadingTcpServer((host, port), Handler)
    actual_port = srv.server_address[1]
    thread = threading.Thread(target=srv.serve_forever, name=f"adapter-{adapter.name}", daemon=True)
    thread.start()
    return srv, thread, actual_port


def serve_stdio(adapter: Adapter) -> None:
    """Runs an adapter in line-delimited JSON stdin/stdout mode (`ext:cmd:...`)."""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception:
            resp = {"choice": 0, "nodes": 0, "error": "invalid json"}
        else:
            resp = adapter.handle_request(req)
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()

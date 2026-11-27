"""Unit and integration tests for Playmat external AI adapters and benchmark harness."""

from __future__ import annotations

import http.server
import json
import os
import sys
import tempfile
import threading
import unittest

RL_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
REPO = os.path.abspath(os.path.join(RL_DIR, "..", ".."))
if RL_DIR not in sys.path:
    sys.path.insert(0, RL_DIR)

from adapters import build_adapter, is_adapter_spec  # noqa: E402
from adapters.external_engines import convert_txt_to_forge_dck, probe_engines  # noqa: E402
from adapters.gemini_decision import GeminiDecisionAdapter  # noqa: E402
from adapters.jev import JevAdapter  # noqa: E402
from adapters.llm import LlmAdapter, _extract_choice_json  # noqa: E402
from adapters.open_ais import ForgeHeuristicAdapter, HttpAgentAdapter, McpBridgeAdapter  # noqa: E402
from benchmark_harness import bradley_terry_elo  # noqa: E402


SAMPLE_REQ = {
    "protocol": "playmat-ext-v1",
    "kind": "decide",
    "seat": 0,
    "turn": 3,
    "step": "PRECOMBAT_MAIN",
    "active_player": "you",
    "query_kind": "priority",
    "query_text": "Play a land, cast a spell, activate an ability, or pass priority.",
    "state": {
        "turn": 3,
        "step": "PRECOMBAT_MAIN",
        "active_player": "you",
        "you": {
            "life": 18,
            "lands_played": 0,
            "library_count": 48,
            "mana_pool": {"W": 0, "U": 0, "B": 0, "R": 0, "G": 0, "C": 0},
            "hand": [
                {"id": 10, "name": "Mountain", "mana_cost": "", "mana_value": 0, "types": ["Land"], "subtypes": ["Mountain"]},
                {"id": 11, "name": "Lightning Strike", "mana_cost": "{1}{R}", "mana_value": 2, "types": ["Instant"], "subtypes": []},
            ],
            "battlefield": [
                {"id": 1, "name": "Mountain", "mana_cost": "", "mana_value": 0, "types": ["Land"], "tapped": False},
                {"id": 2, "name": "Mountain", "mana_cost": "", "mana_value": 0, "types": ["Land"], "tapped": False},
            ],
            "graveyard": [],
            "exile": [],
            "counters": [],
        },
        "opponent": {
            "life": 15,
            "lands_played": 0,
            "hand_count": 4,
            "library_count": 47,
            "mana_pool": {"W": 0, "U": 0, "B": 0, "R": 0, "G": 0, "C": 0},
            "battlefield": [
                {"id": 20, "name": "Savannah Lions", "mana_cost": "{W}", "mana_value": 1, "types": ["Creature"], "power": 2, "toughness": 1, "tapped": False},
            ],
            "graveyard": [],
            "exile": [],
            "counters": [],
        },
        "stack": [],
        "combat": {"attackers": [], "blockers": []},
    },
    "candidates": [
        {"index": 0, "id": "c0", "kind": "pass_or_no", "label": "Pass priority", "is_quick": True},
        {"index": 1, "id": "c1", "kind": "obj", "label": "Play land: Mountain", "card": "Mountain", "obj_id": 10, "is_quick": False},
        {"index": 2, "id": "c2", "kind": "obj", "label": "Cast Lightning Strike {1}{R} (from Hand)", "card": "Lightning Strike", "obj_id": 11, "is_quick": False},
    ],
}


def sample_python_policy(req):
    return {"choice": len(req["candidates"]) - 1, "nodes": 1}


class TestAdapters(unittest.TestCase):
    def test_spec_detection(self):
        self.assertTrue(is_adapter_spec("jev:typesafe/jev-1.13"))
        self.assertTrue(is_adapter_spec("llm:openrouter/anthropic/claude-opus-4.6"))
        self.assertTrue(is_adapter_spec("open:forge-heuristic"))
        self.assertTrue(is_adapter_spec("http://127.0.0.1:9000/decide"))
        self.assertFalse(is_adapter_spec("host@s6"))
        self.assertFalse(is_adapter_spec("rust-quick"))

    def test_jev_payload_and_response_parsing(self):
        jev = JevAdapter("typesafe/jev-1.13", with_value=True)
        payload = jev.build_payload(SAMPLE_REQ)
        self.assertEqual(payload["model"], "typesafe/jev-1.13")
        self.assertIn("state", payload)
        self.assertNotIn("hand", payload["state"]["opponent"])
        self.assertEqual(payload["questions"]["action"]["type"], "choice")
        self.assertEqual(set(payload["questions"]["action"]["criteria"].keys()), {"c0", "c1", "c2"})
        self.assertEqual(payload["questions"]["win_outlook"]["type"], "score")

        fake_api_resp = {
            "model": "typesafe/jev-1.13-20260917",
            "answers": {
                "action": {
                    "type": "choice",
                    "choice": "c1",
                    "probabilities": {"c0": 0.05, "c1": 0.85, "c2": 0.10},
                    "confidence": 0.85,
                },
                "win_outlook": {"type": "score", "score": 3.0, "confidence": 0.7},
            },
            "usage": {"input_tokens": 250, "output_tokens": 30, "cost": 0.000012},
        }
        parsed = jev.parse_response(fake_api_resp, SAMPLE_REQ["candidates"])
        self.assertEqual(parsed["choice"], 1)
        self.assertAlmostEqual(parsed["value"], 0.75)
        self.assertAlmostEqual(sum(parsed["probabilities"]), 1.0, places=4)
        self.assertAlmostEqual(parsed["cost_usd"], 0.000012)

    def test_jev_mock_picks_land(self):
        jev = build_adapter("jev:mock")
        res = jev.handle_request(SAMPLE_REQ)
        self.assertEqual(res["choice"], 1)
        self.assertEqual(jev.telemetry.decisions, 1)

    def test_gemini_decision_payload_and_response(self):
        gd = GeminiDecisionAdapter("gemini-decision-1", with_value=True)
        payload = gd.build_payload(SAMPLE_REQ)
        self.assertEqual(payload["model"], "gemini-decision-1")
        self.assertIsInstance(payload["input"], str)
        self.assertEqual(payload["questions"]["action"]["type"], "choice")
        fake_resp = {
            "answers": {
                "action": {
                    "type": "choice",
                    "choice": "c1",
                    "probabilities": {"c0": 0.01, "c1": 0.96, "c2": 0.03},
                    "confidence": 0.96,
                }
            },
            "usage": {"total_input_tokens": 200, "total_output_tokens": 0},
        }
        parsed = gd.parse_response(fake_resp, SAMPLE_REQ["candidates"])
        self.assertEqual(parsed["choice"], 1)
        self.assertEqual(parsed["output_tokens"], 0)
        self.assertAlmostEqual(parsed["cost_usd"], 200 * 0.10 / 1_000_000.0)
        gd_mock = build_adapter("gemini-decision:mock")
        self.assertEqual(gd_mock.handle_request(SAMPLE_REQ)["choice"], 1)

    def test_llm_json_extraction_and_local_openai_compat(self):
        self.assertEqual(_extract_choice_json('{"choice": 2, "reason": "bolt"}', 3), (2, "bolt"))
        self.assertEqual(_extract_choice_json('```json\n{"choice": "c1", "reason": "land"}\n```', 3), (1, "land"))
        self.assertIsNone(_extract_choice_json('{"choice": 99}', 3))

        class MockOpenAIHandler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                length = int(self.headers.get("Content-Length", 0))
                _ = self.rfile.read(length)
                body = json.dumps({
                    "choices": [{"message": {"content": '{"choice": 2, "reason": "remove threat"}'}}],
                    "usage": {"prompt_tokens": 200, "completion_tokens": 20, "cost": 0.001},
                }).encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, format, *args):
                pass

        srv = http.server.HTTPServer(("127.0.0.1", 0), MockOpenAIHandler)
        t = threading.Thread(target=srv.serve_forever, daemon=True)
        t.start()
        try:
            port = srv.server_address[1]
            llm = LlmAdapter("openai-compat", "qwen3-test", base_url=f"http://127.0.0.1:{port}/v1")
            res = llm.handle_request(SAMPLE_REQ)
            self.assertEqual(res["choice"], 2)
            self.assertEqual(llm.telemetry.input_tokens, 200)
            self.assertAlmostEqual(llm.telemetry.cost_usd, 0.001)
        finally:
            srv.shutdown()
            srv.server_close()

    def test_forge_heuristic_and_http_agent_and_python_plugin(self):
        fh = ForgeHeuristicAdapter()
        self.assertEqual(fh.handle_request(SAMPLE_REQ)["choice"], 1)  # plays land first

        py = build_adapter("python:test_adapters:sample_python_policy")
        self.assertEqual(py.handle_request(SAMPLE_REQ)["choice"], 2)

        class MockAgentHandler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                length = int(self.headers.get("Content-Length", 0))
                req = json.loads(self.rfile.read(length))
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"choice": 1, "nodes": len(req["candidates"])}).encode("utf-8"))

            def log_message(self, format, *args):
                pass

        srv = http.server.HTTPServer(("127.0.0.1", 0), MockAgentHandler)
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        try:
            agent = HttpAgentAdapter(f"http://127.0.0.1:{srv.server_address[1]}/decide")
            res = agent.handle_request(SAMPLE_REQ)
            self.assertEqual(res["choice"], 1)
            self.assertEqual(res["nodes"], 3)
        finally:
            srv.shutdown()
            srv.server_close()

    def test_mcp_bridge_adapter(self):
        script = (
            "import sys, json\n"
            "for line in sys.stdin:\n"
            "  msg = json.loads(line)\n"
            "  if msg.get('method') == 'initialize':\n"
            "    print(json.dumps({'jsonrpc':'2.0','id':msg['id'],'result':{'ok':True}}), flush=True)\n"
            "  else:\n"
            "    print(json.dumps({'jsonrpc':'2.0','id':msg['id'],'result':{'content':[{'type':'text','text':'{\"choice\": 2}'}]}}), flush=True)\n"
        )
        with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False, encoding="utf-8") as f:
            f.write(script)
            script_path = f.name
        mcp = McpBridgeAdapter(f"{sys.executable} {script_path}")
        try:
            res = mcp.handle_request(SAMPLE_REQ)
            self.assertEqual(res["choice"], 2)
        finally:
            mcp.close()
            os.remove(script_path)

    def test_forge_dck_conversion_and_probe_and_bt(self):
        sample_deck = os.path.join(REPO, "Mage.Server", "career-src", "pioneer", "act2-red-burn.txt")
        if os.path.exists(sample_deck):
            with tempfile.TemporaryDirectory() as td:
                out_path = os.path.join(td, "burn.dck")
                convert_txt_to_forge_dck(sample_deck, out_path)
                with open(out_path, encoding="utf-8") as fh:
                    content = fh.read()
                self.assertIn("[metadata]", content)
                self.assertIn("[Main]", content)

        probed = probe_engines()
        self.assertIn("playmat_rust", probed)
        self.assertIn("forge_cli", probed)

        board = bradley_terry_elo(["strong", "mid", "random"], [(0, 2, 18, 0, 2), (1, 2, 12, 0, 8), (0, 1, 14, 0, 6)], "random")
        self.assertEqual(board[0]["name"], "strong")
        self.assertEqual(board[-1]["name"], "random")
        self.assertAlmostEqual(board[-1]["elo"], 1000.0)


if __name__ == "__main__":
    unittest.main()

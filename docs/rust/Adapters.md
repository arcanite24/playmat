# External AI Adapters & Benchmark Harness

Playmat's `DecisionPolicy` trait (`rust/crates/playmat-ai/src/decision.rs`) now includes `PolicyKind::External`
(`rust/crates/playmat-ai/src/external.rs`), allowing any external AI to play full Archbench constructed duels,
tactical suites, and `pro-decisions-1` directly inside the Rust rules engine over a strict imperfect-information
protocol.

## Protocol (`playmat-ext-v1`)

`ExternalPolicy` speaks line-delimited JSON (NDJSON) over either:

- **TCP**: `ext:tcp://127.0.0.1:18500` (append `?obs=1` to include Playmat's numeric `obs::Obs` tensors for ONNX or
  PyTorch models; append `?all_queries=1` to receive raw `PlayMana` / `MultiAmount` queries instead of delegating
  land tapping to Playmat's exact quick payer).
- **Subprocess stdio**: `ext:cmd:python3 -m adapters.server --spec open:forge-heuristic --stdio`.

Fairness guarantee: `external::build_request` never serializes the opponent's hidden hand cards, library contents or
order, or unseen face-down exile. `external::tests::external_state_shows_nothing_the_seat_cant_see` resamples all
hidden cards and asserts the serialized JSON request is byte-identical.

Environment knobs:

- `PLAYMAT_EXT_TIMEOUT_MS` (default `60000`): per-decision socket read/write timeout.
- `PLAYMAT_EXT_OBS=1`: attach numeric `obs` tensors to every request.
- `PLAYMAT_EXT_ALL_QUERIES=1`: forward mana-tapping and damage-division queries.
- `PLAYMAT_EXT_DEBUG=1`: log transport or JSON parse errors to stderr before falling back to quick answers.

## Shipped Adapters (`rust/rl/adapters/`)

All adapters use Python stdlib HTTP by default (no mandatory SDK installs) and enrich visible cards with official rules
text from `rust/crates/playmat-engine/cards/views.json`.

| Adapter Spec | Target AI / System | Notes |
|---|---|---|
| `gemini-decision:gemini-decision-1` | Google Gemini Decision 1 (`choice` + optional `score`) | Gemini Decisions API (`POST /v1beta/decisions`, `GEMINI_API_KEY`); single prefill pass, 0 output tokens (`$0.10 / 1M` input tokens), calibrated probabilities |
| `jev:typesafe/jev-1.13` | TypeSafe AI Jev (`choice` + optional `score`) | OpenRouter Decisions API (`POST /api/alpha/decisions`, `OPENROUTER_API_KEY` or `JEV_API_KEY`); records choice probabilities, confidence, and USD cost |
| `llm:vertex-maas/<pub>/<model>` | Serverless Vertex AI MaaS (`deepseek-ai/deepseek-v4.1-flash-maas`, `zai-org/glm-5.2-maas`, `google/gemini-3.5-flash`, `google/gemini-3.1-pro-preview`, `google/gemma-4-26b-a4b-it-maas`, `openai/gpt-oss-120b-maas`) | Uses `gcloud auth print-access-token` on the global OpenAI-compatible Vertex MaaS endpoint (serverless pay-per-token, no self-hosting) |
| `llm:openrouter/<model>` | Claude, GPT, Gemini, DeepSeek, Qwen, Llama, GLM, Kimi, Grok | Uses `OPENROUTER_API_KEY`; tracks latency, tokens, cost, and fallback rate |
| `llm:openai/<model>` | OpenAI Chat Completions | Uses `OPENAI_API_KEY` |
| `llm:anthropic/<model>` | Anthropic Messages API | Uses `ANTHROPIC_API_KEY` |
| `llm:gemini/<model>` | Google Gemini API | Uses `GEMINI_API_KEY` |
| `llm:ollama/<model>` | Local open-weight LLMs (Ollama) | `$0 / game` local inference at `OLLAMA_HOST` (default `127.0.0.1:11434`) |
| `llm:vllm/<model>` | Local/hosted vLLM or OpenAI-compatible server | Uses `OPENAI_BASE_URL` or `--base-url` |
| `open:forge-heuristic` | Forge-style `canPlayAI()` greedy heuristic | Also supports `open:greedy-aggro` and `open:greedy-control` |
| `onnx:/path/to/model.onnx` | External ONNX policy/value network | Runs via `onnxruntime` on Playmat's `?obs=1` tensors |
| `python:pkg.mod:ClassOrFn` | Custom Python / PyTorch / MageZero / MTG-Causal-RL policy | Append `?obs=1` to receive numeric tensors |
| `http://host:port/decide` | Any external HTTP JSON agent (Java/Kotlin/Go/Node/Rust) | Receives `playmat-ext-v1` JSON via POST |
| `mcp:<command>` | mage-bench-style MCP / JSON-RPC 2.0 stdio agent | Calls `tools/call` (`choose_action`) with board text and legal choices |

External engine wrappers (`rust/rl/adapters/external_engines.py`) also provide Archbench `.txt` to Forge `.dck`
conversion, Forge CLI `sim` runner (`FORGE_JAR`), XMage Java MAD (`RustAiGameTest`) runner, and local engine probing.

## Running Benchmarks (`rust/rl/benchmark_harness.py`)

Build the Rust benchmark binaries once:

```bash
cd rust && cargo build --release -p playmat-ai --bin archbench --bin tactics
```

Probe local engines and CLIs:

```bash
python3 rust/rl/benchmark_harness.py --mode probe
```

Run constructed duels (spawns local TCP adapter servers automatically):

```bash
python3 rust/rl/benchmark_harness.py --mode duels --suite archbench/constructed-2 --deals 10 --jobs 4 \
  --entrant host=host@s6 \
  --entrant quick=rust-quick \
  --entrant forge=open:forge-heuristic \
  --entrant jev=jev:typesafe/jev-1.13 \
  --out-json results_duels.json
```

Run `pro-decisions-1` or `tactics-4` with Jev, frontier LLMs, or local Ollama models and publish straight to the site:

```bash
OPENROUTER_API_KEY=... python3 rust/rl/benchmark_harness.py --mode puzzles --suite archbench/pro-decisions-1 --seeds 2 \
  --entrant host=host@s6 \
  --entrant jev=jev:typesafe/jev-1.13 \
  --entrant opus=llm:openrouter/anthropic/claude-opus-4.6 \
  --entrant qwen=llm:ollama/qwen3:32b \
  --out-json results_pro.json \
  --publish-to-site
```

Standalone adapter server (for manual `archbench`, `tactics`, or `ai-game` runs):

```bash
python3 -m adapters.server --spec jev:typesafe/jev-1.13 --port 18500
cd rust && cargo run --release -p playmat-ai --bin tactics -- \
  --suite ../archbench/pro-decisions-1 --seeds 1 --entrants host@s6,ext:tcp://127.0.0.1:18500
```

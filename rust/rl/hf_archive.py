"""Archive and upload Playmat AI checkpoints and compacted self-play data to public Hugging Face repos.

Reads `HF_TOKEN` from the environment or `<repo>/.env` (never prints the token).

Examples:
    python hf_archive.py --checkpoints runs/loop5 runs/bt-tx788k-all10 runs/bt-tx4m-all10
    python hf_archive.py --data data/loop5-r1 data/loop5-r2 --compact
    python hf_archive.py --cardtext
"""

import argparse
import os
import sys
import tempfile
import time

from huggingface_hub import HfApi

import data as data_mod

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))

DEFAULT_MODELS_REPO = "nerijs/playmat-ai-checkpoints"
DEFAULT_DATASET_REPO = "nerijs/playmat-ai-selfplay"

CKPT_ALLOW = [
    "*.pmnet",
    "*.pt",
    "*.json",
    "*.jsonl",
    "**/net.pmnet",
    "**/net.pt",
    "**/arch.json",
    "**/report.json",
    "**/rounds.jsonl",
]

DATA_ALLOW = [
    "merged.*",
]


def load_token(env_path=None):
    tok = os.environ.get("HF_TOKEN") or os.environ.get("HUGGING_FACE_HUB_TOKEN")
    if tok:
        return tok.strip()
    candidates = [env_path] if env_path else [
        os.path.join(REPO, ".env"),
        os.path.join(HERE, ".env"),
        os.path.expanduser("~/.hf_token"),
    ]
    for p in candidates:
        if not p or not os.path.exists(p):
            continue
        with open(p, "r", encoding="utf-8") as f:
            for line in f:
                s = line.strip()
                if not s or s.startswith("#"):
                    continue
                if "=" in s:
                    k, v = s.split("=", 1)
                    if k.strip() in ("HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"):
                        return v.strip().strip("\"'")
                elif len(s) >= 20 and s.startswith("hf_"):
                    return s
    sys.exit("HF_ARCHIVE error: HF_TOKEN not found in environment or .env")


def ensure_repo(api, repo_id, repo_type):
    api.create_repo(repo_id=repo_id, repo_type=repo_type, private=False, exist_ok=True)


def upload_checkpoint_dir(api, repo_id, local_dir, remote_name=None):
    local_dir = os.path.abspath(local_dir)
    name = remote_name or os.path.basename(os.path.normpath(local_dir))
    t0 = time.time()
    print(f"HF_ARCHIVE uploading checkpoints {local_dir} -> {repo_id}/checkpoints/{name}", flush=True)
    api.upload_folder(
        folder_path=local_dir,
        path_in_repo=f"checkpoints/{name}",
        repo_id=repo_id,
        repo_type="model",
        allow_patterns=CKPT_ALLOW,
        commit_message=f"Upload checkpoints: {name}",
    )
    print(f"HF_ARCHIVE done checkpoints/{name} in {time.time() - t0:.1f} s", flush=True)


def upload_data_dir(api, repo_id, local_dir, remote_name=None, compact=False):
    local_dir = os.path.abspath(local_dir)
    name = remote_name or os.path.basename(os.path.normpath(local_dir))
    if compact:
        data_mod.compact_dir(local_dir)
    merged = [f for f in os.listdir(local_dir) if f.startswith("merged.")]
    if not merged:
        print(f"HF_ARCHIVE skipping {local_dir}: no merged.* files (pass --compact first)", flush=True)
        return
    total_gb = sum(os.path.getsize(os.path.join(local_dir, f)) for f in merged) / 2**30
    t0 = time.time()
    print(f"HF_ARCHIVE uploading data {local_dir} ({len(merged)} files, {total_gb:.2f} GB) -> {repo_id}/data/{name}", flush=True)
    api.upload_folder(
        folder_path=local_dir,
        path_in_repo=f"data/{name}",
        repo_id=repo_id,
        repo_type="dataset",
        allow_patterns=DATA_ALLOW,
        commit_message=f"Upload compacted self-play data: {name} ({total_gb:.2f} GB)",
    )
    print(f"HF_ARCHIVE done data/{name} in {time.time() - t0:.1f} s", flush=True)


def upload_cardtext(api, repo_id, data_root=None):
    root = data_root or os.path.join(HERE, "data")
    for fname in ("cardtext.npy", "cardtext_sem.npy"):
        p = os.path.join(root, fname)
        if not os.path.exists(p):
            continue
        t0 = time.time()
        mb = os.path.getsize(p) / 2**20
        print(f"HF_ARCHIVE uploading {p} ({mb:.1f} MB) -> {repo_id}/cardtext/{fname}", flush=True)
        api.upload_file(
            path_or_fileobj=p,
            path_in_repo=f"cardtext/{fname}",
            repo_id=repo_id,
            repo_type="model",
            commit_message=f"Upload {fname}",
        )
        print(f"HF_ARCHIVE done cardtext/{fname} in {time.time() - t0:.1f} s", flush=True)


def upload_readme(api, repo_id, repo_type, content):
    with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8") as f:
        f.write(content)
        tmp = f.name
    try:
        api.upload_file(
            path_or_fileobj=tmp,
            path_in_repo="README.md",
            repo_id=repo_id,
            repo_type=repo_type,
            commit_message="Update README.md",
        )
    finally:
        os.remove(tmp)


MODELS_README = """---
license: mit
tags:
  - reinforcement-learning
  - magic-the-gathering
  - alphazero
  - ismcts
  - transformer
---

# Playmat AI Checkpoints

Neural network checkpoints (`.pmnet` for Rust inference + `.pt` PyTorch weights + `arch.json` + training `report.json`) for the Playmat Magic: The Gathering self-play AI.

## Layout

- `checkpoints/loop5/` — Baseline `loop5` transformer rounds (`r1`..`r10`, 730k params, trained on `netsearch` self-play)
- `checkpoints/bt-tx788k-all10/` — 788k-param Transformer (`val_mlp=1`, regularized 2-layer value MLP, trained on all 10 compacted `loop5` rounds / 593k games, held-out `value_logloss=0.4175`, `policy_ce=0.1099`, `accuracy_not_quick=89.1%`)
- `checkpoints/bt-tx4m-all10/` — 4.5M-param Transformer (`d=256, heads=8, layers=6, ff=640, val_mlp=1, gelu=1, text=2` with BGE-small card text embeddings, trained on all 10 compacted `loop5` rounds / 593k games, held-out `value_logloss=0.4278`)
- `checkpoints/loop6/` — Quiescence Terminal-Disciplined Value-Leaf Gumbel-ISMCTS self-play rounds (recording `qval.f16`)
- `cardtext/` — Precomputed card text embeddings (`cardtext.npy` hash/TF-IDF, `cardtext_sem.npy` BGE-small + 24 MTG tactical tags)
"""

DATASET_README = """---
license: mit
tags:
  - reinforcement-learning
  - magic-the-gathering
  - self-play
---

# Playmat AI Self-Play Dataset

Compacted binary self-play shards (`merged.*`, 13 files per round) generated by `playmat-ai` (`rust/crates/playmat-ai/src/bin/selfplay.rs`) across the 42-deck `constructed@2` benchmark suite.

## Shard Columns (`merged.*`)

- `merged.meta.i32`: `[game, seat, turn, chosen, n_ent, n_act, n_hid, n_pool, qkind, tag]` (`int32`)
- `merged.glob.f16`: Global board features (`GLOB_F=65`, `float16`)
- `merged.ents.f16` / `merged.ent_ids.u16`: Visible entities (stack, battlefields, own hand; `ENT_F=49`) + hashed card bucket IDs
- `merged.acts.f16` / `merged.act_ids.u16` / `merged.target.f16`: Candidate actions (`ACT_F=65`), card IDs, and search-improved policy targets
- `merged.hid.f16` / `merged.hid_ids.u16`: Opponent hidden hand entities (for asymmetric critic / belief supervision)
- `merged.pool.u16` / `merged.pool_hand.u8`: Unseen card pool + in-hand binary labels for the belief head
- `merged.logp.f32`: Behavior policy log-probability
- `merged.qval.f16`: Root search Q-value estimates (`loop6+`; `NaN` on `loop5` shards)
- `merged.games.npz`: Per-game outcomes (`games`, `results`, `turns`)

## Rounds

- `data/loop5-r1` .. `data/loop5-r10`: ~650k games (~31M decisions) generated with `netsearch` + `eval 2` rollouts
- `data/loop6-r*`: Games generated with Quiescence Terminal-Disciplined Value-Leaf Gumbel-ISMCTS (includes `qval.f16`)
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models-repo", default=DEFAULT_MODELS_REPO)
    ap.add_argument("--dataset-repo", default=DEFAULT_DATASET_REPO)
    ap.add_argument("--env", default=None, help="path to .env containing HF_TOKEN")
    ap.add_argument("--checkpoints", nargs="*", default=[], help="checkpoint folders to upload")
    ap.add_argument("--data", nargs="*", default=[], help="self-play data folders to upload")
    ap.add_argument("--compact", action="store_true", help="compact uncompacted data dirs before uploading")
    ap.add_argument("--cardtext", action="store_true", help="upload cardtext*.npy embeddings")
    ap.add_argument("--cardtext-dir", default=None, help="directory containing cardtext*.npy")
    ap.add_argument("--init-readmes", action="store_true", help="create/update README.md cards on both repos")
    args = ap.parse_args()

    tok = load_token(args.env)
    api = HfApi(token=tok)

    if args.checkpoints or args.cardtext or args.init_readmes:
        ensure_repo(api, args.models_repo, "model")
    if args.data or args.init_readmes:
        ensure_repo(api, args.dataset_repo, "dataset")

    if args.init_readmes:
        upload_readme(api, args.models_repo, "model", MODELS_README)
        upload_readme(api, args.dataset_repo, "dataset", DATASET_README)
        print(f"HF_ARCHIVE updated READMEs on {args.models_repo} and {args.dataset_repo}", flush=True)

    if args.cardtext:
        upload_cardtext(api, args.models_repo, args.cardtext_dir)

    for d in args.checkpoints:
        upload_checkpoint_dir(api, args.models_repo, d)

    for d in args.data:
        upload_data_dir(api, args.dataset_repo, d, compact=args.compact)


if __name__ == "__main__":
    main()

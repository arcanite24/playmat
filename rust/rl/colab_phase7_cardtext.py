"""Colab GPU generator & uploader for Phase 7 `1088d` Full-Sheet Card Embeddings (`cardtext_v2_32k.npy` & `cardtext_v2_4k.npy`).

Usage on Colab GPU (`colab run --gpu L4 colab_phase7_cardtext.py` or `--gpu A100`):
- Encodes all `16,483` verified cards in `card_registry_32k.txt` (+ fallback views in `views.json.gz`) using
  `BAAI/bge-large-en-v1.5` (`1024d`, `max_length=512`, L2-normalized) + `64` deterministic card-sheet scalars = `1088d`.
- Writes `cardtext_v2_32k.npy` (`[32768, 1088]`, float16, 0.0% collisions) and `cardtext_v2_4k.npy` (`[4096, 1088]`,
  float16, L2-renormalized bucket averages with zero `constructed-2` overwrites).
- Uploads `cardtext_v2_32k.npy`, `cardtext_v2_4k.npy`, and `card_registry_32k.txt` to Hugging Face
  `nerijs/playmat-ai-checkpoints` under `cardtext/`.
"""

import argparse
import gzip
import json
import os
import shutil
import subprocess
import sys
import time


def sh(cmd):
    print("COLAB_CT $", " ".join(cmd), flush=True)
    subprocess.check_call(cmd)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    ap.add_argument("--encoder", default="BAAI/bge-large-en-v1.5")
    ap.add_argument("--batch-size", type=int, default=256)
    ap.add_argument("--max-length", type=int, default=512)
    ap.add_argument("--views-gz", default=None, help="optional local path to views.json.gz")
    ap.add_argument("--registry", default=None, help="optional local path to card_registry_32k.txt")
    ap.add_argument("--code-repo-url", default="https://github.com/arcanite24/playmat.git")
    ap.add_argument("--code-branch", default="master")
    ap.add_argument("--no-upload", action="store_true")
    args = ap.parse_args()

    t0 = time.time()
    sh([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet", "transformers", "accelerate"])

    import numpy as np
    import torch
    from huggingface_hub import HfApi, snapshot_download

    tok = os.environ.get("HF_TOKEN")
    if not tok and not args.no_upload:
        sys.exit("COLAB_CT error: HF_TOKEN env var not set")

    print(f"COLAB_CT device: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'}", flush=True)
    work = "/content/playmat_cardtext"
    os.makedirs(work, exist_ok=True)

    here = os.path.dirname(os.path.abspath(__file__))
    local_ct = os.path.join(here, "cardtext.py")
    local_views = args.views_gz or os.path.join(here, "..", "crates", "playmat-engine", "cards", "views.json.gz")
    local_reg = args.registry or os.path.join(here, "..", "crates", "playmat-ai", "src", "card_registry_32k.txt")

    if not (os.path.exists(local_ct) and os.path.exists(local_views) and os.path.exists(local_reg)):
        # Pull from HF models repo first if uploaded, else sparse-checkout the 3 files from fork repo
        try:
            snapshot_download(
                repo_id=args.models_repo,
                repo_type="model",
                local_dir=work,
                allow_patterns=["code/rl/cardtext.py", "cardtext/card_registry_32k.txt", "cardtext/views.json.gz"],
                token=tok,
            )
        except Exception:
            pass
        cand_ct = os.path.join(work, "code", "rl", "cardtext.py")
        cand_reg = os.path.join(work, "cardtext", "card_registry_32k.txt")
        cand_vw = os.path.join(work, "cardtext", "views.json.gz")
        if not (os.path.exists(cand_ct) and os.path.exists(cand_reg) and os.path.exists(cand_vw)):
            repo_dir = os.path.join(work, "repo")
            if not os.path.exists(repo_dir):
                sh(["git", "clone", "--depth", "1", "--filter=blob:none", "--sparse", "--branch", args.code_branch, args.code_repo_url, repo_dir])
                sh(["git", "-C", repo_dir, "sparse-checkout", "set", "rust/rl", "rust/crates/playmat-ai/src", "rust/crates/playmat-engine/cards"])
            if not os.path.exists(local_ct):
                local_ct = os.path.join(repo_dir, "rust", "rl", "cardtext.py")
            if not os.path.exists(local_reg):
                local_reg = os.path.join(repo_dir, "rust", "crates", "playmat-ai", "src", "card_registry_32k.txt")
            if not os.path.exists(local_views):
                local_views = os.path.join(repo_dir, "rust", "crates", "playmat-engine", "cards", "views.json.gz")
        else:
            local_ct, local_reg, local_views = cand_ct, cand_reg, cand_vw

    sys.path.insert(0, os.path.dirname(os.path.abspath(local_ct)))
    import cardtext  # noqa: E402

    print(f"COLAB_CT loading views from {local_views} and registry from {local_reg}...", flush=True)
    with gzip.open(local_views, "rt", encoding="utf-8") as f:
        views = json.load(f)

    t_enc = time.time()
    t32, t4 = cardtext.build_v2_tables(
        views,
        registry_path=local_reg,
        model_name=args.encoder,
        batch_size=args.batch_size,
        max_length=args.max_length,
    )
    print(f"COLAB_CT encoded tables in {time.time() - t_enc:.1f} s: 32k={t32.shape}, 4k={t4.shape}", flush=True)

    assert t32.shape == (32768, 1088) and np.isfinite(t32).all(), "invalid t32 table"
    assert t4.shape == (4096, 1088) and np.isfinite(t4).all(), "invalid t4 table"
    # Verify row 0 is zero and rows 1..=16483 have unit-norm BGE slices
    assert float(np.abs(t32[0]).max()) == 0.0, "slot 0 must be zero padding"
    reg_names, _ = cardtext.load_registry_32k(local_reg)
    reg_norms = np.linalg.norm(t32[1 : len(reg_names) + 1, :1024], axis=1)
    assert np.allclose(reg_norms, 1.0, atol=1e-3), f"BGE slice not unit-norm: min={reg_norms.min()} max={reg_norms.max()}"

    out_dir = os.path.join(work, "upload_cardtext")
    os.makedirs(out_dir, exist_ok=True)
    p32 = os.path.join(out_dir, "cardtext_v2_32k.npy")
    p4 = os.path.join(out_dir, "cardtext_v2_4k.npy")
    preg = os.path.join(out_dir, "card_registry_32k.txt")
    np.save(p32, t32.astype(np.float16))
    np.save(p4, t4.astype(np.float16))
    shutil.copyfile(local_reg, preg)
    print(
        f"COLAB_CT saved {p32} ({os.path.getsize(p32)/1e6:.1f} MB) & {p4} ({os.path.getsize(p4)/1e6:.1f} MB)",
        flush=True,
    )

    if not args.no_upload:
        api = HfApi(token=tok)
        api.upload_folder(
            folder_path=out_dir,
            path_in_repo="cardtext",
            repo_id=args.models_repo,
            repo_type="model",
            commit_message=f"Phase 7 1088d full-sheet BGE-large cardtext v2 tables (32k & 4k)",
        )
        print(f"COLAB_CT uploaded cardtext/ to {args.models_repo} in {time.time() - t0:.1f} s", flush=True)


if __name__ == "__main__":
    main()

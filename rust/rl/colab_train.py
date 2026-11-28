"""Self-contained Colab GPU training runner (`colab run --gpu A100 --high-mem --timeout 0 colab_train.py ...`).

Pulls code, cardtext embeddings, and compacted self-play shards straight from Hugging Face, trains on Colab GPU,
and uploads the finished checkpoint (`.pmnet`, `.pt`, `arch.json`, `report.json`) back to Hugging Face.
Reads `HF_TOKEN` from environment (never prints it).
"""

import argparse
import os
import subprocess
import sys
import time


def sh(cmd, cwd=None):
    print("COLAB $", " ".join(cmd), flush=True)
    p = subprocess.Popen(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    for line in p.stdout:
        print(line, end="", flush=True)
    p.wait()
    if p.returncode != 0:
        sys.exit(f"COLAB failed ({p.returncode}): {' '.join(cmd)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", required=True, help="checkpoint folder name in HF models repo (e.g. obsv2-r1)")
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    ap.add_argument("--dataset-repo", default="nerijs/playmat-ai-selfplay")
    ap.add_argument("--rounds", nargs="+", required=True, help="data/<round> names in dataset repo (oldest -> newest)")
    ap.add_argument("--init-ckpt", default=None, help="optional checkpoints/<name> in models repo to warm-start from")
    ap.add_argument("--oracle-ckpt", default=None, help="optional checkpoints/<name> in models repo for oracle distillation")
    ap.add_argument("--oracle-mix", type=float, default=0.0)
    ap.add_argument("--arch", default="tx")
    ap.add_argument("--tx", default="d=192,heads=6,layers=4,ff=512,dim=32,trunk=192,val_mlp=1,gelu=1,glob_f=81,rel=1,act_ent=1,text=2")
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--bs", type=int, default=2048)
    ap.add_argument("--lr", type=float, default=1.2e-3)
    ap.add_argument("--drop", type=float, default=0.1)
    ap.add_argument("--card-id-drop", type=float, default=0.0)
    ap.add_argument("--val-prob", type=float, default=0.50)
    ap.add_argument("--val-wd", type=float, default=5e-2)
    ap.add_argument("--value-weight", type=float, default=1.0)
    ap.add_argument("--aux-weight", type=float, default=0.15)
    ap.add_argument("--act-q-weight", type=float, default=0.2)
    ap.add_argument("--recency-decay", type=float, default=0.92)
    ap.add_argument("--recency-half-life", type=int, default=0)
    ap.add_argument("--oracle-mode", action="store_true")
    ap.add_argument("--cache-oracle", type=int, default=1)
    ap.add_argument("--upgrade-glob-f", type=int, default=0)
    ap.add_argument("--upgrade-val-bins", type=int, default=0)
    ap.add_argument("--threat-weight", type=float, default=0.0)
    ap.add_argument("--opt-pol-weight", type=float, default=0.0)
    ap.add_argument("--opt", default="adamw")
    ap.add_argument("--muon-lr", type=float, default=0.012)
    ap.add_argument("--q-mix", type=float, default=0.4)
    ap.add_argument("--td-lambda", type=float, default=0.0)
    ap.add_argument("--ema", type=float, default=0.995)
    ap.add_argument("--policy-recent", type=int, default=1)
    ap.add_argument("--pol-old-weight", type=float, default=0.0)
    ap.add_argument("--max-old-per-dir", type=int, default=1500000)
    ap.add_argument("--samples-per-epoch", type=int, default=3000000)
    ap.add_argument("--stage-chunk", type=int, default=1500000)
    ap.add_argument("--gpu-data-gb", type=float, default=18.0)
    ap.add_argument("--compile", action="store_true", help="enable torch.compile on Colab CUDA")
    ap.add_argument("--no-refit-value", action="store_true")
    args = ap.parse_args()

    t0 = time.time()
    sh([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"])
    import torch
    from huggingface_hub import HfApi, snapshot_download

    tok = os.environ.get("HF_TOKEN")
    if not tok:
        sys.exit("COLAB error: HF_TOKEN env var not set")

    print(f"COLAB GPU: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'}", flush=True)
    eff_gpu_gb = args.gpu_data_gb
    if torch.cuda.is_available():
        gb = torch.cuda.get_device_properties(0).total_memory / 2**30
        eff_gpu_gb = min(args.gpu_data_gb, max(0.0, round(gb - 14.0, 1)))
        print(f"COLAB VRAM: {gb:.1f} GB (effective --gpu-data-gb={eff_gpu_gb})", flush=True)

    work = "/content/playmat"
    os.makedirs(work, exist_ok=True)

    # 1. Pull rl code + cardtext embeddings from models repo
    print("COLAB downloading rl code and cardtext embeddings...", flush=True)
    snapshot_download(
        repo_id=args.models_repo,
        repo_type="model",
        local_dir=work,
        allow_patterns=["code/rl/train.py", "code/rl/model.py", "code/rl/data.py", "code/rl/cardtext.py", "code/rl/hf_archive.py", "cardtext/*"],
        token=tok,
    )
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src = os.path.join(work, "cardtext", fn)
        dst = os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    init_dir = None
    if args.init_ckpt:
        print(f"COLAB downloading init checkpoint checkpoints/{args.init_ckpt}...", flush=True)
        snapshot_download(
            repo_id=args.models_repo,
            repo_type="model",
            local_dir=work,
            allow_patterns=[f"checkpoints/{args.init_ckpt}/*"],
            token=tok,
        )
        init_dir = os.path.join(work, "checkpoints", args.init_ckpt)

    oracle_dir = None
    if args.oracle_ckpt:
        print(f"COLAB downloading oracle checkpoint checkpoints/{args.oracle_ckpt}...", flush=True)
        snapshot_download(
            repo_id=args.models_repo,
            repo_type="model",
            local_dir=work,
            allow_patterns=[f"checkpoints/{args.oracle_ckpt}/*"],
            token=tok,
        )
        oracle_dir = os.path.join(work, "checkpoints", args.oracle_ckpt)

    # 2. Pull compacted self-play rounds from dataset repo
    allow_data = [f"data/{r}/merged.*" for r in args.rounds]
    print(f"COLAB downloading {len(args.rounds)} rounds from {args.dataset_repo}: {args.rounds}", flush=True)
    t_dl = time.time()
    snapshot_download(
        repo_id=args.dataset_repo,
        repo_type="dataset",
        local_dir=work,
        allow_patterns=allow_data,
        token=tok,
    )
    print(f"COLAB downloaded data in {time.time() - t_dl:.1f} s", flush=True)

    out_dir = os.path.join(work, "out", args.name)
    cmd = [
        sys.executable, "-u", "train.py",
        "--out", out_dir,
        "--no-critic",
        "--bs", str(args.bs),
        "--amp", "1",
        "--epochs", str(args.epochs),
        "--lr", str(args.lr),
        "--drop", str(args.drop),
        "--card-id-drop", str(args.card_id_drop),
        "--val-prob", str(args.val_prob),
        "--val-wd", str(args.val_wd),
        "--value-weight", str(args.value_weight),
        "--aux-weight", str(args.aux_weight),
        "--act-q-weight", str(args.act_q_weight),
        "--threat-weight", str(args.threat_weight),
        "--opt-pol-weight", str(args.opt_pol_weight),
        "--opt", str(args.opt),
        "--muon-lr", str(args.muon_lr),
        "--recency-decay", str(args.recency_decay),
        "--q-mix", str(args.q_mix),
        "--td-lambda", str(args.td_lambda),
        "--ema", str(args.ema),
        "--policy-recent", str(args.policy_recent),
        "--pol-old-weight", str(args.pol_old_weight),
        "--max-old-per-dir", str(args.max_old_per_dir),
        "--samples-per-epoch", str(args.samples_per_epoch),
        "--stage-chunk", str(args.stage_chunk),
        "--gpu-data-gb", str(eff_gpu_gb),
        "--cache-oracle", str(args.cache_oracle),
    ]
    if args.recency_half_life > 0:
        cmd += ["--recency-half-life", str(args.recency_half_life)]
    if args.compile:
        cmd += ["--compile"]
    if args.no_refit_value:
        cmd += ["--no-refit-value"]
    if args.oracle_mode:
        cmd += ["--oracle-mode"]
    if oracle_dir and args.oracle_mix > 0:
        cmd += ["--oracle-dir", oracle_dir, "--oracle-mix", str(args.oracle_mix)]
    if init_dir:
        cmd += ["--init", init_dir]
        if args.upgrade_glob_f > 0:
            cmd += ["--upgrade-glob-f", str(args.upgrade_glob_f)]
        if args.upgrade_val_bins > 0:
            cmd += ["--upgrade-val-bins", str(args.upgrade_val_bins)]
    else:
        cmd += ["--arch", args.arch, "--tx", args.tx]
    for r in args.rounds:
        cmd += ["--data", os.path.join(work, "data", r)]

    sh(cmd, cwd=rl_dir)

    # 3. Upload trained checkpoint directly back to HF models repo
    api = HfApi(token=tok)
    print(f"COLAB uploading {out_dir} -> {args.models_repo}/checkpoints/{args.name}", flush=True)
    api.upload_folder(
        folder_path=out_dir,
        path_in_repo=f"checkpoints/{args.name}",
        repo_id=args.models_repo,
        repo_type="model",
        allow_patterns=["*.pmnet", "*.pt", "*.json", "*.jsonl"],
        commit_message=f"Colab trained checkpoint: {args.name}",
    )
    print(f"COLAB ALL DONE ({args.name}) in {time.time() - t0:.1f} s", flush=True)


if __name__ == "__main__":
    main()

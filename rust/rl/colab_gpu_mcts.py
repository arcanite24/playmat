"""Colab GPU-Batched MCTS Runner (Phase 3 Pillar 1).

Runs our portable Linux AVX2 `archbench` / `tactics` binaries directly on a Colab A100/L4 VM
talking to `infer_server.py` over localhost `tcp://127.0.0.1:18400` using Virtual-Loss Batched MCTS (`vloss=8`).
"""

import argparse
import os
import socket
import stat
import subprocess
import sys
import time


def wait_port(host, port, timeout=180.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.5)
    raise TimeoutError(f"infer_server did not open {host}:{port} within {timeout}s")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default="phase3-r15b-5m-aux4")
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    ap.add_argument("--suite", default="constructed-2")
    ap.add_argument("--deals", type=int, default=100)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--iters", type=int, default=1024)
    ap.add_argument("--vloss", type=int, default=8)
    ap.add_argument("--samples", type=int, default=8)
    ap.add_argument("--extra-turns", type=int, default=3)
    ap.add_argument("--opponent", default="host@s6")
    ap.add_argument("--compile", action="store_true")
    ap.add_argument("--tactics", default="tactics-4")
    args = ap.parse_args()

    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ.get("HF_TOKEN")
    if not tok:
        sys.exit("HF_TOKEN missing")
    print(f"GPU_MCTS device={torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'} cpus={os.cpu_count()}", flush=True)

    work = "/content/playmat_gpu_mcts"
    os.makedirs(work, exist_ok=True)
    snapshot_download(
        repo_id=args.models_repo,
        repo_type="model",
        local_dir=work,
        allow_patterns=[
            "code/rl/infer_server.py", "code/rl/model.py", "code/rl/train.py", "code/rl/data.py", "code/rl/cardtext.py",
            "cardtext/*",
            "bin/linux-x86_64/archbench", "bin/linux-x86_64/tactics",
            "archbench/*",
            "Mage.Server/career-src/*",
            f"checkpoints/{args.ckpt}/*",
        ],
        token=tok,
    )
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    os.makedirs("/home/playmat", exist_ok=True)
    if not os.path.exists("/home/playmat/mage"):
        os.symlink(work, "/home/playmat/mage")
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src = os.path.join(work, "cardtext", fn)
        dst = os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    archbench_bin = os.path.join(work, "bin", "linux-x86_64", "archbench")
    tactics_bin = os.path.join(work, "bin", "linux-x86_64", "tactics")
    for b in (archbench_bin, tactics_bin):
        os.chmod(b, os.stat(b).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    ckpt_dir = os.path.join(work, "checkpoints", args.ckpt)
    srv_cmd = [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"), "--dir", ckpt_dir, "--port", "18400", "--amp", "1"]
    if args.compile:
        srv_cmd.append("--compile")
    srv_log = open("/content/infer_server.log", "w")
    srv = subprocess.Popen(srv_cmd, cwd=rl_dir, stdout=srv_log, stderr=subprocess.STDOUT)
    try:
        wait_port("127.0.0.1", 18400, timeout=180.0)
        print("INFER_SERVER_READY on tcp://127.0.0.1:18400", flush=True)

        env = dict(os.environ)
        env["PLAYMAT_GPU_BATCH"] = "64"
        env["PLAYMAT_GPU_WAIT_US"] = "250"

        gpu_net = "tcp://127.0.0.1:18400"
        cpu_net = os.path.join(ckpt_dir, "net.pmnet")

        # 1. Quick throughput comparison: GPU-Batched (vloss=8) vs CPU AVX2 on 8 deals (16 games) at iters=512
        print("=== 1. THROUGHPUT BENCHMARK: GPU-BATCHED (vloss=8) vs CPU AVX2 (5.07M, 16 games @ iters=512) ===", flush=True)
        for label, net_url, vl in [("GPU_VLOSS8", gpu_net, args.vloss), ("CPU_AVX2", cpu_net, 1)]:
            ent = f"cfg:policy=ismcts+net={net_url}+hide=1+iters=512+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2+belief_ms=1+vloss={vl}"
            t0 = time.time()
            pr = subprocess.run([archbench_bin, "--suite", os.path.join(work, "archbench", args.suite), "--deals", "8", "--jobs", str(args.jobs), "--entrants", f"{ent},host@s6"], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            print(pr.stdout, flush=True)
            if pr.returncode != 0:
                raise RuntimeError(f"archbench failed ({pr.returncode})")
            print(f"SPEED {label}: {time.time() - t0:.1f} s", flush=True)

        # 2. Tactics-4 at iters=1024 GPU-batched
        if args.tactics:
            print(f"=== 2. {args.tactics} GPU-BATCHED ({args.ckpt} @ iters=1024, vloss={args.vloss}) ===", flush=True)
            ent_t = f"cfg:policy=ismcts+net={gpu_net}+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=3+belief_ms=1+vloss={args.vloss}"
            pr = subprocess.run([tactics_bin, "--suite", os.path.join(work, "archbench", args.tactics), "--seeds", "4", "--entrants", ent_t], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True); print(pr.stdout, flush=True); assert pr.returncode == 0

        # 3. Main deep GPU-batched fair match
        print(f"=== 3. MAIN GPU-BATCHED MATCH ({args.ckpt} @ iters={args.iters}, vloss={args.vloss}, {args.deals*2} games on {args.suite}) ===", flush=True)
        ent_main = f"cfg:policy=ismcts+net={gpu_net}+hide=1+iters={args.iters}+samples={args.samples}+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns={args.extra_turns}+belief_ms=1+vloss={args.vloss}+budget=24000+playout_cap=600"
        pr = subprocess.run([archbench_bin, "--suite", os.path.join(work, "archbench", args.suite), "--deals", str(args.deals), "--jobs", str(args.jobs), "--entrants", f"{ent_main},{args.opponent}"], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True); print(pr.stdout, flush=True); assert pr.returncode == 0
    finally:
        srv.terminate()
        try:
            srv.wait(timeout=5)
        except subprocess.TimeoutExpired:
            srv.kill()
        srv_log.close()
        print("=== INFER_SERVER LOG ===", flush=True)
        print(open("/content/infer_server.log").read()[-2000:], flush=True)


if __name__ == "__main__":
    main()

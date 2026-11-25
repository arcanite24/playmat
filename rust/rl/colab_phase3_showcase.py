"""Grand Finale Colab GPU-Batched MCTS Showcase (`phase3-final-5m-v3-soup` @ iters=1024, vloss=8, hide=1)."""
import argparse, os, socket, subprocess, sys, time


def wait_port(host, port, timeout=60.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.25)
    return False


def run_cmd(cmd, env):
    pr = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(pr.stdout, flush=True)
    if pr.returncode != 0:
        raise RuntimeError(f"Command failed ({pr.returncode}): {' '.join(cmd)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default="phase3-final-5m-v3-soup")
    ap.add_argument("--c2-deals", type=int, default=80)
    ap.add_argument("--ood-deals", type=int, default=50)
    ap.add_argument("--iters", type=int, default=1024)
    ap.add_argument("--vloss", type=int, default=8)
    ap.add_argument("--jobs", type=int, default=12)
    args = ap.parse_args()

    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"SHOWCASE device={torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'} cpus={os.cpu_count()}", flush=True)

    work = "/content/playmat_showcase"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    os.makedirs("/home/playmat", exist_ok=True)
    if not os.path.exists("/home/playmat/mage"):
        os.symlink(work, "/home/playmat/mage")

    snapshot_download(
        repo_id="nerijs/playmat-ai-checkpoints",
        repo_type="model",
        local_dir=work,
        allow_patterns=[
            "bin/linux-x86_64/*",
            "archbench/*",
            "Mage.Server/career-src/*",
            "code/rl/*.py",
            "cardtext/*",
            f"checkpoints/{args.ckpt}/*",
        ],
        token=tok,
    )
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src = os.path.join(work, "cardtext", fn)
        dst = os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    archbench_bin = os.path.join(work, "bin", "linux-x86_64", "archbench")
    tactics_bin = os.path.join(work, "bin", "linux-x86_64", "tactics")
    os.chmod(archbench_bin, 0o755)
    os.chmod(tactics_bin, 0o755)

    ckpt_dir = os.path.join(work, "checkpoints", args.ckpt)
    srv_log = open(os.path.join(work, "infer_server.log"), "w")
    srv = subprocess.Popen(
        [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"), "--dir", ckpt_dir, "--port", "18400", "--amp", "1"],
        cwd=rl_dir, stdout=srv_log, stderr=subprocess.STDOUT,
    )
    try:
        assert wait_port("127.0.0.1", 18400, 60.0), "infer_server failed to start"
        print("INFER_SERVER_READY on tcp://127.0.0.1:18400", flush=True)
        env = dict(os.environ, PLAYMAT_REPO=work, RAYON_NUM_THREADS=str(args.jobs))
        base = (
            f"cfg:policy=ismcts+net=tcp://127.0.0.1:18400+iters={args.iters}+samples=8"
            f"+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1"
            f"+width=24+consider=16+extra_turns=3+belief_ms=1+vloss={args.vloss}+budget=24000+playout_cap=600"
        )

        print(f"=== 1. tactics-4 FULL-INFO (hide=0) & FAIR (hide=1) ({args.ckpt} @ iters={args.iters}, vloss={args.vloss}) ===", flush=True)
        run_cmd([tactics_bin, "--suite", os.path.join(work, "archbench", "tactics-4"), "--seeds", "4", "--entrants", f"{base}+hide=1,{base}+hide=0"], env)

        print(f"=== 2. SHOWCASE constructed-2 ({args.c2_deals * 2} fair games, {args.ckpt} @ iters={args.iters}, vloss={args.vloss} vs omniscient host@s6) ===", flush=True)
        run_cmd([archbench_bin, "--suite", os.path.join(work, "archbench", "constructed-2"), "--deals", str(args.c2_deals), "--jobs", str(args.jobs), "--entrants", f"{base}+hide=1,host@s6"], env)

        print(f"=== 3. SHOWCASE constructed-ood-1 ({args.ood_deals * 2} fair OOD games, {args.ckpt} @ iters={args.iters}, vloss={args.vloss} vs omniscient host@s6) ===", flush=True)
        run_cmd([archbench_bin, "--suite", os.path.join(work, "archbench", "constructed-ood-1"), "--deals", str(args.ood_deals), "--jobs", str(args.jobs), "--entrants", f"{base}+hide=1,host@s6"], env)
    finally:
        srv.terminate()
        try:
            srv.wait(timeout=5)
        except Exception:
            srv.kill()
        srv_log.close()
        print("\n=== INFER_SERVER LOG ===", flush=True)
        print(open(os.path.join(work, "infer_server.log")).read(), flush=True)


if __name__ == "__main__":
    main()

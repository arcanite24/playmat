"""Pre-Phase-4 GPU Benchmark: micro-batching speedup A/B + 5.07M vs 10.10M @ iters=1024 fair H2H shootout."""
import argparse, os, socket, subprocess, sys, time


def wait_port(host, port, timeout=60.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.2)
    return False


def run_cmd(cmd, env):
    t0 = time.time()
    pr = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dt = time.time() - t0
    print(pr.stdout, flush=True)
    if pr.returncode != 0:
        raise RuntimeError(f"Command failed ({pr.returncode}): {' '.join(cmd)}")
    return dt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--deals", type=int, default=50, help="deals for 10M vs 5M H2H (2*deals games)")
    ap.add_argument("--jobs", type=int, default=12)
    args = ap.parse_args()

    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"PRE4_BENCH device={torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'} cpus={os.cpu_count()}", flush=True)

    work = "/content/playmat_pre4"
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
            "code/rl/infer_server.py",
            "code/rl/model.py",
            "code/rl/cardtext.py",
            "code/rl/train.py",
            "code/rl/data.py",
            "cardtext/*",
            "checkpoints/phase3-final-5m-v3-soup/*",
            "checkpoints/phase3-final-10m-v3-soup/*",
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

    archbench = os.path.join(work, "bin", "linux-x86_64", "archbench")
    os.chmod(archbench, 0o755)
    env = dict(os.environ, PLAYMAT_REPO=work, RAYON_NUM_THREADS=str(args.jobs))
    c5 = os.path.join(work, "checkpoints", "phase3-final-5m-v3-soup")
    c10 = os.path.join(work, "checkpoints", "phase3-final-10m-v3-soup")
    suite_c2 = os.path.join(work, "archbench", "constructed-2")

    # 1. THROUGHPUT A/B: Old Serialized (max_batch=1, wait_ms=0) vs New Micro-Batched (max_batch=512, wait_ms=0.6)
    print("=== 1. THROUGHPUT A/B: SERIALIZED vs CROSS-THREAD MICRO-BATCHING (5.07M @ iters=512, 16 games) ===", flush=True)
    for label, mb, wms, port in [("SERIALIZED", 1, 0.0, 18410), ("MICROBATCH_512", 512, 0.6, 18411)]:
        logp = os.path.join(work, f"srv_{label}.log")
        lf = open(logp, "w")
        srv = subprocess.Popen(
            [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
             "--dir", c5, "--port", str(port), "--amp", "1", "--max-batch", str(mb), "--wait-ms", str(wms)],
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        try:
            assert wait_port("127.0.0.1", port, 60.0), f"{label} server failed"
            ent = (f"cfg:policy=ismcts+net=tcp://127.0.0.1:{port}+hide=1+iters=512+samples=8+cpuct=1000"
                   f"+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2+belief_ms=1+vloss=8")
            dt = run_cmd([archbench, "--suite", suite_c2, "--deals", "8", "--jobs", str(args.jobs), "--entrants", f"{ent},host@s6"], env)
            print(f"SPEED {label}: {dt:.1f} s", flush=True)
        finally:
            srv.terminate()
            srv.wait(timeout=5)
            lf.close()
            print(f"--- {label} SERVER LOG ---\n" + open(logp).read(), flush=True)

    # 2. DEEP-SEARCH SCALING SHOOTOUT: 10.10M vs 5.07M @ iters=1024, vloss=8, hide=1 (100 games on constructed-2)
    print(f"=== 2. DEEP-SEARCH SHOOTOUT: phase3-final-10m-v3-soup vs phase3-final-5m-v3-soup @ iters=1024 ({args.deals*2} fair games) ===", flush=True)
    lf5 = open(os.path.join(work, "srv_5m.log"), "w")
    lf10 = open(os.path.join(work, "srv_10m.log"), "w")
    s5 = subprocess.Popen(
        [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
         "--dir", c5, "--port", "18400", "--amp", "1", "--max-batch", "512", "--wait-ms", "0.6"],
        cwd=rl_dir, stdout=lf5, stderr=subprocess.STDOUT,
    )
    s10 = subprocess.Popen(
        [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
         "--dir", c10, "--port", "18401", "--amp", "1", "--max-batch", "512", "--wait-ms", "0.6"],
        cwd=rl_dir, stdout=lf10, stderr=subprocess.STDOUT,
    )
    try:
        assert wait_port("127.0.0.1", 18400, 60.0) and wait_port("127.0.0.1", 18401, 60.0)
        base = "+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=3+belief_ms=1+vloss=8+budget=24000+playout_cap=600"
        e5 = f"cfg:policy=ismcts+net=tcp://127.0.0.1:18400{base}"
        e10 = f"cfg:policy=ismcts+net=tcp://127.0.0.1:18401{base}"
        dt = run_cmd([archbench, "--suite", suite_c2, "--deals", str(args.deals), "--jobs", str(args.jobs), "--entrants", f"{e10},{e5}"], env)
        print(f"SHOOTOUT_DONE in {dt:.1f} s", flush=True)
    finally:
        for s, lf in [(s5, lf5), (s10, lf10)]:
            s.terminate()
            s.wait(timeout=5)
            lf.close()
        print("--- 5M SERVER LOG ---\n" + open(os.path.join(work, "srv_5m.log")).read(), flush=True)
        print("--- 10M SERVER LOG ---\n" + open(os.path.join(work, "srv_10m.log")).read(), flush=True)


if __name__ == "__main__":
    main()

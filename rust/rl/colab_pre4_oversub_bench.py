"""Pre-Phase-4 GPU Oversubscription & torch.compile Throughput Benchmark (12 vs 24 jobs, vloss 8 vs 16, 5M vs 10M)."""
import os, socket, subprocess, sys, time


def wait_port(host, port, timeout=120.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.25)
    return False


def main():
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"OVERSUB_BENCH device={torch.cuda.get_device_name(0)} cpus={os.cpu_count()}", flush=True)

    work = "/content/playmat_ov"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    os.makedirs("/home/playmat", exist_ok=True)
    if not os.path.exists("/home/playmat/mage"): os.symlink(work, "/home/playmat/mage")
    snapshot_download(
        repo_id="nerijs/playmat-ai-checkpoints",
        repo_type="model",
        local_dir=work,
        allow_patterns=[
            "bin/linux-x86_64/archbench",
            "archbench/constructed-2/*",
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
    c5 = os.path.join(work, "checkpoints", "phase3-final-5m-v3-soup")
    c10 = os.path.join(work, "checkpoints", "phase3-final-10m-v3-soup")
    suite = os.path.join(work, "archbench", "constructed-2")

    configs = [
        ("5M_J12_VL8", c5, 12, 8, 0, 18501),
        ("5M_J24_VL16", c5, 24, 16, 0, 18502),
        ("5M_J32_VL16", c5, 32, 16, 0, 18503),
        ("10M_J24_VL16", c10, 24, 16, 0, 18504),
        ("10M_J32_VL16", c10, 32, 16, 0, 18505),
    ]
    for label, ckpt, jobs, vloss, comp, port in configs:
        logp = os.path.join(work, f"srv_{label}.log")
        lf = open(logp, "w")
        srv = subprocess.Popen(
            [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
             "--dir", ckpt, "--port", str(port), "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5"] + (["--compile"] if comp else []),
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        try:
            assert wait_port("127.0.0.1", port, 90.0), f"{label} server failed"
            ent = (f"cfg:policy=ismcts+net=tcp://127.0.0.1:{port}+hide=1+iters=512+samples=8+cpuct=1000"
                   f"+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2+belief_ms=1+vloss={vloss}")
            env = dict(os.environ, PLAYMAT_REPO=work, RAYON_NUM_THREADS=str(jobs))
            t0 = time.time()
            pr = subprocess.run([archbench, "--suite", suite, "--deals", "8", "--jobs", str(jobs), "--entrants", f"{ent},host@s6"],
                                env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            dt = time.time() - t0
            print(f"=== {label}: {dt:.1f} s (rc={pr.returncode}) ===", flush=True)
            for ln in pr.stdout.splitlines():
                if pr.returncode != 0 or "ARCHBENCH" in ln:
                    print("  " + ln, flush=True)
        finally:
            srv.terminate()
            srv.wait(timeout=5)
            lf.close()
            print(f"  SERVER: " + " | ".join(l.strip() for l in open(logp).read().splitlines()), flush=True)


if __name__ == "__main__":
    main()

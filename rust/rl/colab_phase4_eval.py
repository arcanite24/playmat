"""Phase 4 Fair (hide=1) GPU-Batched Evaluation Runner for Colab A100/L4.

Runs Bradley-Terry anchor gauntlets (iters=256) or 10.10M deep-search showcases (iters=1024)
using native Rust archbench over local PyTorch AMP infer_server.py instances.
"""
import argparse, os, socket, subprocess, sys, time


def wait_port(host, port, timeout=120.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.25)
    return False


def build_spec(preset, vloss, extra_turns):
    base_256 = (
        f"+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+vloss={vloss}"
    )
    base_1024 = (
        f"+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+vloss={vloss}"
    )
    if preset == "bt6_256":
        return [
            ("p4_5m_v4_soup", "phase4-final-5m-v4-soup", base_256 + "+fpu_vnode=1+solver=1"),
            ("p4_1.3m_v4_soup", "phase4-final-1.3m-v4-soup", base_256 + "+fpu_vnode=1+solver=1"),
            ("p3_5m_v3_matched_search", "phase3-final-5m-v3-soup", base_256 + "+fpu_vnode=1+solver=1"),
            ("p3_5m_v3_base", "phase3-final-5m-v3-soup", base_256),
            ("p3_1.3m_v3_base", "phase3-final-1.3m-v3-soup", base_256),
            ("host@s6", None, None),
        ]
    if preset == "showcase10m_3way":
        return [
            ("p4_10m_v4_soup@1024", "phase4-final-10m-v4-soup", base_1024 + "+fpu_vnode=1+solver=1"),
            ("p3_10m_v3_base@1024", "phase3-final-10m-v3-soup", base_1024),
            ("host@s6", None, None),
        ]
    if preset == "showcase10m_matched":
        return [
            ("p4_10m_v4_soup@1024", "phase4-final-10m-v4-soup", base_1024 + "+fpu_vnode=1+solver=1"),
            ("p3_10m_v3_matched@1024", "phase3-final-10m-v3-soup", base_1024 + "+fpu_vnode=1+solver=1"),
            ("p4_5m_v4_soup@1024", "phase4-final-5m-v4-soup", base_1024 + "+fpu_vnode=1+solver=1"),
        ]
    raise ValueError(f"Unknown preset: {preset}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preset", required=True, choices=["bt6_256", "showcase10m_3way", "showcase10m_matched"])
    ap.add_argument("--suite", default="constructed-2")
    ap.add_argument("--deals", type=int, default=30)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--vloss", type=int, default=16)
    ap.add_argument("--extra-turns", type=int, default=2)
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    args = ap.parse_args()

    t0 = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(
        f"PHASE4_EVAL preset={args.preset} suite={args.suite} deals={args.deals} "
        f"jobs={args.jobs} vloss={args.vloss} device={torch.cuda.get_device_name(0)} cpus={os.cpu_count()}",
        flush=True,
    )

    work = "/content/playmat_p4eval"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    os.makedirs("/home/playmat", exist_ok=True)
    if os.path.islink("/home/playmat/mage") or os.path.exists("/home/playmat/mage"):
        try:
            os.unlink("/home/playmat/mage")
        except OSError:
            pass
    if not os.path.exists("/home/playmat/mage"):
        os.symlink(work, "/home/playmat/mage")

    spec = build_spec(args.preset, args.vloss, args.extra_turns)
    ckpts = list(dict.fromkeys([c for _, c, _ in spec if c is not None]))
    patterns = [
        "bin/linux-x86_64/archbench",
        f"archbench/{args.suite}/*",
        "Mage.Server/career-src/*",
        "code/rl/infer_server.py",
        "code/rl/model.py",
        "code/rl/cardtext.py",
        "code/rl/train.py",
        "code/rl/data.py",
        "cardtext/*",
    ] + [f"checkpoints/{c}/*" for c in ckpts]

    snapshot_download(repo_id=args.models_repo, repo_type="model", local_dir=work, allow_patterns=patterns, token=tok)
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src = os.path.join(work, "cardtext", fn)
        dst = os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    ab_bin = os.path.join(work, "bin", "linux-x86_64", "archbench")
    os.chmod(ab_bin, 0o755)

    procs = []
    ports = {}
    for idx, c in enumerate(ckpts):
        port = 18700 + idx
        ports[c] = port
        logp = os.path.join(work, f"srv_{idx}.log")
        lf = open(logp, "w")
        p = subprocess.Popen(
            [
                sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
                "--dir", os.path.join(work, "checkpoints", c),
                "--port", str(port), "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5",
            ],
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        procs.append((c, p, lf, logp))

    try:
        for c, port in ports.items():
            assert wait_port("127.0.0.1", port, 120.0), f"infer_server failed for {c}"
        print(f"ALL {len(ports)} GPU INFER SERVERS READY", flush=True)

        entrants = []
        print("=== ENTRANT LEGEND ===", flush=True)
        for label, ckpt, flags in spec:
            if ckpt is None:
                e = label
            else:
                e = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[ckpt]}{flags}"
            entrants.append(e)
            print(f"  {label} => {e}", flush=True)
        print("======================", flush=True)

        suite_dir = os.path.join(work, "archbench", args.suite)
        cmd = [
            ab_bin,
            "--suite", suite_dir,
            "--entrants", ",".join(entrants),
            "--deals", str(args.deals),
            "--jobs", str(args.jobs),
        ]
        print("RUN:", " ".join(cmd[:6]), "...", flush=True)
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        out_lines = []
        for line in proc.stdout:
            out_lines.append(line)
            print(line, end="", flush=True)
        rc = proc.wait()
        print(f"ARCHBENCH_RC={rc} elapsed={time.time()-t0:.1f}s", flush=True)
        if rc != 0:
            raise SystemExit(rc)
    finally:
        for c, p, lf, logp in procs:
            p.terminate()
            try:
                p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                p.kill()
            lf.close()
            try:
                lines = open(logp).read().strip().splitlines()
                print(f"SRV {c}: " + " | ".join(lines[-3:]), flush=True)
            except Exception:
                pass


if __name__ == "__main__":
    main()

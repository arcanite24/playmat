"""Phase 6 Fair (hide=1) GPU-Batched Evaluation Runner for Colab A100/L4."""
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
    b256 = (
        f"+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+fpu_vnode=1+solver=1+vloss={vloss}+threat_w=500+playout=2"
    )
    b1024 = (
        f"+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+fpu_vnode=1+solver=1+vloss={vloss}+threat_w=500+playout=2"
    )
    if preset == "p6_r25_sanity":
        return [
            ("p6_r25_10m_cd@256", "phase6-r25-10m-v4-cd", b256),
            ("p6_r25_5m_cd@256", "phase6-r25-5m-v4-cd", b256),
            ("p6pre_5m_carddrop@256", "p6pre-5m-carddrop", b256),
            ("p5_1.3m_v4_soup@256", "phase5-final-1.3m-v4-soup", b256),
        ]
    if preset == "p6_gauntlet_256":
        return [
            ("p6_final_10m_soup@256", "phase6-final-10m-v4-soup", b256),
            ("p6_final_5m_soup@256", "phase6-final-5m-v4-soup", b256),
            ("p6_final_1.3m_soup@256", "phase6-final-1.3m-v4-soup", b256),
            ("p6pre_5m_carddrop@256", "p6pre-5m-carddrop", b256),
            ("p5_1.3m_v4_soup@256", "phase5-final-1.3m-v4-soup", b256),
            ("host@s6", None, None),
        ]
    if preset == "p6_frontier_1024":
        return [
            ("p6_final_10m_soup@1024", "phase6-final-10m-v4-soup", b1024),
            ("p6_final_5m_soup@1024", "phase6-final-5m-v4-soup", b1024),
            ("p5_10m_v4_soup@1024", "phase5-final-10m-v4-soup", b1024),
            ("host@s6", None, None),
        ]
    if preset == "p6_stress_256":
        return [
            ("p6_final_5m_soup@256", "phase6-final-5m-v4-soup", b256),
            ("p6_final_10m_soup@256", "phase6-final-10m-v4-soup", b256),
            ("host@s6", None, None),
        ]
    raise ValueError(f"Unknown preset: {preset}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preset", required=True)
    ap.add_argument("--suite", default="constructed-2,constructed-ood-1")
    ap.add_argument("--deals", type=int, default=20)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--vloss", type=int, default=16)
    ap.add_argument("--extra-turns", type=int, default=2)
    args = ap.parse_args()

    suites = [s.strip() for s in args.suite.split(",") if s.strip()]
    t0 = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"PHASE6_EVAL preset={args.preset} suites={suites} deals={args.deals} jobs={args.jobs} device={torch.cuda.get_device_name(0)}", flush=True)

    work = "/content/playmat_p6eval"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    for base_dir, link_path in [("/home/playmat", "/home/playmat/mage"), ("/home/brandonalamgarcianeri", "/home/brandonalamgarcianeri/playmat")]:
        os.makedirs(base_dir, exist_ok=True)
        if os.path.islink(link_path) or os.path.exists(link_path):
            try: os.unlink(link_path)
            except OSError: pass
        if not os.path.exists(link_path):
            os.symlink(work, link_path)

    spec = build_spec(args.preset, args.vloss, args.extra_turns)
    ckpts = list(dict.fromkeys([c for _, c, _ in spec if c is not None]))
    patterns = [
        "bin/linux-x86_64/archbench",
        "Mage.Server/career-src/*",
        "code/rl/infer_server.py", "code/rl/model.py", "code/rl/cardtext.py", "code/rl/train.py", "code/rl/data.py",
        "cardtext/*",
    ] + [f"archbench/{s}/*" for s in suites] + [f"checkpoints/{c}/*" for c in ckpts]
    snapshot_download(repo_id="nerijs/playmat-ai-checkpoints", repo_type="model", local_dir=work, allow_patterns=patterns, token=tok)
    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src, dst = os.path.join(work, "cardtext", fn), os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    ab_bin = os.path.join(work, "bin", "linux-x86_64", "archbench")
    os.chmod(ab_bin, 0o755)

    procs, ports = [], {}
    for idx, c in enumerate(ckpts):
        port = 18710 + idx
        ports[c] = port
        lf = open(os.path.join(work, f"srv_{idx}.log"), "w")
        p = subprocess.Popen(
            [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
             "--dir", os.path.join(work, "checkpoints", c),
             "--port", str(port), "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5"],
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        procs.append((c, p, lf))

    try:
        for c, port in ports.items():
            assert wait_port("127.0.0.1", port, 120.0), f"failed {c}"
        entrants = []
        print("=== ENTRANT LEGEND ===", flush=True)
        for label, ckpt, flags in spec:
            e = label if ckpt is None else f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[ckpt]}{flags}"
            entrants.append(e)
            print(f"  {label} => {e}", flush=True)
        env = dict(os.environ, PLAYMAT_GAME_TIMEOUT_SECS="2400")
        for s in suites:
            print(f"=== RUNNING SUITE {s} (deals={args.deals}, jobs={args.jobs}) ===", flush=True)
            pr = subprocess.Popen(
                [ab_bin, "--suite", os.path.join(work, "archbench", s),
                 "--entrants", ",".join(entrants), "--deals", str(args.deals), "--jobs", str(args.jobs)],
                cwd=work, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1,
            )
            for line in pr.stdout:
                print(line, end="", flush=True)
            rc = pr.wait()
            print(f"ARCHBENCH_RC={rc} suite={s} elapsed={time.time()-t0:.1f}s", flush=True)
            if rc != 0:
                raise SystemExit(rc)
    finally:
        for c, p, lf in procs:
            p.terminate()
            try: p.wait(timeout=5)
            except: p.kill()
            lf.close()


if __name__ == "__main__":
    main()

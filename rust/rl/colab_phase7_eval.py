"""Phase 7 Fair (hide=1) Single-Server Multi-Model GPU Evaluation Runner (supports mixed 4k/32k Tx & TxV2 + ActQ FPU)."""
import argparse, os, socket, subprocess, sys, time


def wait_unix(path, timeout=240.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        if os.path.exists(path):
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.settimeout(1.0)
                s.connect(path)
                s.close()
                return True
            except OSError:
                pass
        time.sleep(0.25)
    return False


def build_spec(preset, vloss, extra_turns):
    base256 = (
        f"+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+fpu_vnode=1+solver=1+vloss={vloss}+threat_w=500+playout=2+prog_worlds=1"
    )
    base1024 = (
        f"+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000"
        f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        f"+extra_turns={extra_turns}+belief_ms=1+fpu_vnode=1+solver=1+vloss={vloss}+threat_w=500+playout=2+prog_worlds=1"
    )
    if preset == "w2_gate_256":
        return [
            ("p7_txv2_51m_32k_aq@256", "p7-w2-txv2-51m-32k", base256 + "+act_q_fpu=1"),
            ("p7_txv2_51m_4k@256", "p7-w2-txv2-51m", base256),
            ("p7_txv2_28m_32k_aq@256", "p7-w2-txv2-28m-32k", base256 + "+act_q_fpu=1"),
            ("p6pre_5m_carddrop@256", "p6pre-5m-carddrop", base256),
            ("p6_final_10m_soup@256", "phase6-final-10m-v4-soup", base256),
            ("host@s6", None, None),
        ]
    if preset == "w2_gate_28m_ab":
        return [
            ("p7_txv2_28m_32k_aq@256", "p7-w2-txv2-28m-32k", base256 + "+act_q_fpu=1"),
            ("p7_txv2_28m_4k@256", "p7-w2-txv2-28m", base256),
            ("p6pre_5m_carddrop@256", "p6pre-5m-carddrop", base256),
            ("host@s6", None, None),
        ]
    if preset == "w2_frontier_1024":
        return [
            ("p7_txv2_51m_32k_aq@1024", "p7-w2-txv2-51m-32k", base1024 + "+act_q_fpu=1"),
            ("p7_txv2_28m_32k_aq@1024", "p7-w2-txv2-28m-32k", base1024 + "+act_q_fpu=1"),
            ("p6_final_10m_soup@1024", "phase6-final-10m-v4-soup", base1024),
            ("host@s6", None, None),
        ]
    raise ValueError(f"Unknown preset: {preset}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preset", required=True)
    ap.add_argument("--suite", default="constructed-2,constructed-ood-1")
    ap.add_argument("--deals", type=int, default=15)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--vloss", type=int, default=16)
    ap.add_argument("--extra-turns", type=int, default=2)
    ap.add_argument("--puzzles", type=int, default=1)
    ap.add_argument("--compile", type=int, default=0)
    args = ap.parse_args()

    suites = [s.strip() for s in args.suite.split(",") if s.strip()]
    t0 = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"PHASE7_EVAL preset={args.preset} suites={suites} deals={args.deals} jobs={args.jobs} device={torch.cuda.get_device_name(0)}", flush=True)

    work = "/content/playmat_p7eval"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    for base_dir, link_path in [
        ("/home/playmat", "/home/playmat/mage"),
        ("/home/brandonalamgarcianeri", "/home/brandonalamgarcianeri/playmat"),
    ]:
        os.makedirs(base_dir, exist_ok=True)
        if os.path.islink(link_path) or os.path.exists(link_path):
            try:
                os.unlink(link_path)
            except OSError:
                pass
        if not os.path.exists(link_path):
            os.symlink(work, link_path)

    spec = build_spec(args.preset, args.vloss, args.extra_turns)
    ckpts = list(dict.fromkeys([c for _, c, _ in spec if c is not None]))
    patterns = [
        "bin/linux-x86_64/archbench",
        "bin/linux-x86_64/tactics",
        "Mage.Server/career-src/*",
        "archbench/tactics-4/*",
        "archbench/pro-decisions-1/*",
        "code/rl/infer_server.py", "code/rl/model.py", "code/rl/cardtext.py", "code/rl/train.py", "code/rl/data.py",
        "cardtext/*",
    ] + [f"archbench/{s}/*" for s in suites] + [f"checkpoints/{c}/*" for c in ckpts]
    snapshot_download(repo_id="nerijs/playmat-ai-checkpoints", repo_type="model", local_dir=work, allow_patterns=patterns, token=tok)

    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy", "cardtext_v2_4k.npy", "cardtext_v2_32k.npy", "card_registry_32k.txt"):
        src, dst = os.path.join(work, "cardtext", fn), os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)

    ab_bin = os.path.join(work, "bin", "linux-x86_64", "archbench")
    tac_bin = os.path.join(work, "bin", "linux-x86_64", "tactics")
    os.chmod(ab_bin, 0o755)
    os.chmod(tac_bin, 0o755)

    socks = {}
    model_specs = []
    for idx, c in enumerate(ckpts):
        sp = f"/tmp/pm_p7_{idx}.sock"
        if os.path.exists(sp):
            os.unlink(sp)
        socks[c] = sp
        model_specs.append(f"unix://{sp}={os.path.join(work, 'checkpoints', c)}")

    srv_log = os.path.join(work, "infer_srv.log")
    lf = open(srv_log, "w")
    srv_cmd = [
        sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
        "--models", ",".join(model_specs),
        "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5",
    ]
    if args.compile:
        srv_cmd += ["--compile"]
    srv = subprocess.Popen(srv_cmd, cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT)

    try:
        for c, sp in socks.items():
            assert wait_unix(sp, 240.0), f"infer_server failed to bind {sp} for {c}"
        print(f"MULTI-MODEL UNIX SERVER READY ({len(ckpts)} models)", flush=True)

        entrants = []
        print("=== ENTRANT LEGEND ===", flush=True)
        for label, ckpt, flags in spec:
            e = label if ckpt is None else f"cfg:policy=ismcts+net=unix://{socks[ckpt]}{flags}"
            entrants.append(e)
            print(f"  {label} => {e}", flush=True)

        env = dict(os.environ, PLAYMAT_GAME_TIMEOUT_SECS="2400", PLAYMAT_REPO=work)
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

        if args.puzzles:
            p_entrants = [e for e in entrants if e != "host@s6"][:3]
            if p_entrants:
                for psuite in ("tactics-4", "pro-decisions-1"):
                    print(f"=== RUNNING PUZZLES {psuite} (hide=1) ===", flush=True)
                    pr = subprocess.Popen(
                        [tac_bin, "--suite", os.path.join(work, "archbench", psuite), "--entrants", ",".join(p_entrants)],
                        cwd=work, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1,
                    )
                    for line in pr.stdout:
                        print(line, end="", flush=True)
                    pr.wait()
    finally:
        srv.terminate()
        try: srv.wait(timeout=5)
        except Exception: srv.kill()
        lf.close()
        try:
            print("SRV_TAIL:\n" + "\n".join(open(srv_log).read().splitlines()[-20:]), flush=True)
        except Exception:
            pass


if __name__ == "__main__":
    main()

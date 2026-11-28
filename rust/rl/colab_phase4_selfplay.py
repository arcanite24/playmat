"""Phase 4 Deep-Search GPU PFSP Self-Play Runner on Colab A100/L4.

Runs the 10.10M flagship (with --jobs 24, vloss=16, fpu_vnode=1, solver=1, forced_k=200, hide=1)
against a diverse PFSP league (self-play mirror + 5.07M sibling + OOD HL-Gauss specialist + host@s6 anchor),
compacts the Obs-v4 + Hidden-Hand Threat shards, and uploads merged.* directly to Hugging Face.
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--round", required=True, help="dataset round name, e.g. phase4-r19-10m-colab")
    ap.add_argument("--learner-ckpt", required=True, help="HF checkpoint name for flagship 10M learner")
    ap.add_argument("--sib-ckpt", default="pre4-muon004-optpol-5m", help="5M sibling opponent ckpt")
    ap.add_argument("--ood-ckpt", default="pre4-hlgauss21-5m", help="OOD regularized specialist opponent ckpt")
    ap.add_argument("--games", type=int, default=1200)
    ap.add_argument("--iters", type=int, default=512)
    ap.add_argument("--samples", type=int, default=8)
    ap.add_argument("--vloss", type=int, default=16)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--keep", type=int, default=400, help="KataGo playout cap randomization keep rate (per 1000)")
    ap.add_argument("--fast-iters", type=int, default=64)
    ap.add_argument("--seed", type=int, default=19000)
    ap.add_argument("--tag", type=int, default=19)
    ap.add_argument("--hardening", action="store_true", help="Focus on stress-control + stress-bigmana decks (13 decks)")
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    ap.add_argument("--dataset-repo", default="nerijs/playmat-ai-selfplay")
    args = ap.parse_args()

    t0 = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import HfApi, snapshot_download

    tok = os.environ["HF_TOKEN"]
    print(f"PHASE4_SP round={args.round} device={torch.cuda.get_device_name(0)} cpus={os.cpu_count()} games={args.games}", flush=True)

    work = "/content/playmat_p4sp"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    os.makedirs("/home/playmat", exist_ok=True)
    if not os.path.exists("/home/playmat/mage"):
        os.symlink(work, "/home/playmat/mage")

    ckpts = list(dict.fromkeys([args.learner_ckpt, args.sib_ckpt, args.ood_ckpt]))
    patterns = [
        "bin/linux-x86_64/selfplay",
        "archbench/constructed-2/*",
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

    sp_bin = os.path.join(work, "bin", "linux-x86_64", "selfplay")
    os.chmod(sp_bin, 0o755)
    out_dir = os.path.join(work, "shards", args.round)
    os.makedirs(out_dir, exist_ok=True)

    # Start 3 lightweight GPU inference servers on localhost ports (10M learner, 5M sibling, 5M OOD specialist)
    procs = []
    ports = {}
    for idx, c in enumerate(ckpts):
        port = 18600 + idx
        ports[c] = port
        logp = os.path.join(work, f"srv_{idx}.log")
        lf = open(logp, "w")
        p = subprocess.Popen(
            [sys.executable, "-u", os.path.join(rl_dir, "infer_server.py"),
             "--dir", os.path.join(work, "checkpoints", c),
             "--port", str(port), "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5"],
            cwd=rl_dir, stdout=lf, stderr=subprocess.STDOUT,
        )
        procs.append((c, p, lf, logp))

    try:
        for c, port in ports.items():
            assert wait_port("127.0.0.1", port, 120.0), f"infer_server failed for {c}"
        print(f"ALL {len(ports)} GPU INFER SERVERS READY", flush=True)

        common = (
            f"+hide=1+iters={args.iters}+samples={args.samples}+cpuct=1000+leaf_blend=1000"
            f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2"
            f"+belief_ms=1+fpu_vnode=1+solver=1+forced_k=200+vloss={args.vloss}+temp=1000+temp_turn=6"
        )
        opp_common = (
            f"+hide=1+iters={max(256, args.iters // 2)}+samples={args.samples}+cpuct=1000+leaf_blend=1000"
            f"+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16+extra_turns=2"
            f"+belief_ms=1+fpu_vnode=1+solver=1+vloss={args.vloss}+temp=750+temp_turn=6"
        )
        learner = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.learner_ckpt]}{common}"
        sib = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.sib_ckpt]}{opp_common}"
        ood = f"cfg:policy=ismcts+net=tcp://127.0.0.1:{ports[args.ood_ckpt]}{opp_common}"
        suite_path = os.path.join(work, "archbench", "constructed-2")
        if args.hardening:
            import json
            c2 = json.loads(open(os.path.join(suite_path, "suite.json")).read())
            kws = ("control", "spirits", "dimir-midrange", "stompy", "landfall", "vigorbloom", "angels", "dragons", "greasefang")
            sub = [d for d in c2["decks"] if any(k in d["file"] for k in kws)]
            hdir = os.path.join(work, "archbench", "hardening-1")
            os.makedirs(hdir, exist_ok=True)
            open(os.path.join(hdir, "suite.json"), "w").write(json.dumps(dict(c2, suite="hardening@1", decks=sub), indent=2))
            suite_path = hdir
            print(f"HARDENING SUITE READY: {len(sub)} control+bigmana decks", flush=True)
            opponents = f"self*4;{sib}*3;{ood}*1;host@s6*2"
        else:
            opponents = f"self*5;{sib}*3;{ood}*1;host@s6*1"

        cmd = [
            sp_bin,
            "--out", out_dir,
            "--repo", work,
            "--suite", suite_path,
            "--learner", learner,
            "--opponents", opponents,
            "--games", str(args.games),
            "--jobs", str(args.jobs),
            "--keep", str(args.keep),
            "--fast", f"ismcts:{args.fast_iters}",
            "--seed", str(args.seed),
            "--tag", str(args.tag),
        ]
        env = dict(os.environ, PLAYMAT_REPO=work, RAYON_NUM_THREADS=str(args.jobs))
        print("RUN $", " ".join(cmd), flush=True)
        pr = subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        for line in pr.stdout:
            print(line, end="", flush=True)
        pr.wait()
        if pr.returncode != 0:
            sys.exit(f"selfplay failed with rc={pr.returncode}")
    finally:
        for c, p, lf, logp in procs:
            p.terminate()
            try:
                p.wait(timeout=5)
            except Exception:
                p.kill()
            lf.close()
            print(f"--- SERVER {c} ---\n" + open(logp).read(), flush=True)

    # Compact shards into merged.* and upload to HF dataset repo
    print(f"MERGING {out_dir}...", flush=True)
    subprocess.run([sys.executable, "-u", os.path.join(rl_dir, "data.py"), "--compact", out_dir], check=True)
    api = HfApi(token=tok)
    print(f"UPLOADING {out_dir}/merged.* -> {args.dataset_repo}/data/{args.round}", flush=True)
    api.upload_folder(
        folder_path=out_dir,
        path_in_repo=f"data/{args.round}",
        repo_id=args.dataset_repo,
        repo_type="dataset",
        allow_patterns=["merged.*"],
        commit_message=f"Phase 4 deep-search 10M Obs-v4 PFSP shards: {args.round}",
    )
    print(f"PHASE4_SP ALL DONE ({args.round}) in {time.time() - t0:.1f} s", flush=True)


if __name__ == "__main__":
    main()

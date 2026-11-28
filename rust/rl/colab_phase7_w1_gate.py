"""Phase 7 Wave 1 Cloud GPU Verification Gate (Speedup A/B + Zero-Retrain Elo A/B + Train Compile Check)."""
import argparse, os, socket, subprocess, sys, time


def wait_port(host, port, timeout=180.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            with socket.create_connection((host, port), timeout=1.0):
                return True
        except OSError:
            time.sleep(0.25)
    return False


def wait_unix(path, timeout=180.0):
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


def run_cmd(cmd, cwd, env):
    t0 = time.time()
    pr = subprocess.Popen(
        cmd, cwd=cwd, env=env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1,
    )
    out = []
    for line in pr.stdout:
        print(line, end="", flush=True)
        out.append(line)
    rc = pr.wait()
    dt = time.time() - t0
    if rc != 0:
        raise RuntimeError(f"Command failed rc={rc} in {dt:.1f}s: {' '.join(cmd[:6])}")
    return dt, "".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--deals-speed", type=int, default=4)
    ap.add_argument("--deals-ab", type=int, default=10)
    ap.add_argument("--jobs", type=int, default=24)
    ap.add_argument("--compile", type=int, default=0)
    ap.add_argument("--skip-elo-gate", type=int, default=0)
    args = ap.parse_args()

    t_start = time.time()
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"], check=True)
    import torch
    from huggingface_hub import snapshot_download

    tok = os.environ["HF_TOKEN"]
    dev_name = torch.cuda.get_device_name(0)
    print(f"PHASE7_W1_GATE device={dev_name} cpus={os.cpu_count()} compile={args.compile}", flush=True)

    work = "/content/playmat_w1gate"
    os.makedirs(os.path.join(work, "rust", "crates", "playmat-ai"), exist_ok=True)
    for base_dir, link_path in [
        ("/home/playmat", "/home/playmat/mage"),
        ("/home/brandonalamgarcianeri", "/home/brandonalamgarcianeri/playmat"),
        ("/home/brandonalamgarcianeri", "/home/brandonalamgarcianeri/src"),
    ]:
        os.makedirs(base_dir, exist_ok=True)
        if os.path.islink(link_path) or os.path.exists(link_path):
            try:
                os.unlink(link_path)
            except OSError:
                pass
        if not os.path.exists(link_path):
            os.symlink(work, link_path)

    ckpts = [
        "phase6-final-10m-v4-soup",
        "p6pre-5m-carddrop",
        "phase6-final-1.3m-v4-soup",
    ]
    patterns = [
        "bin/linux-x86_64/archbench",
        "bin/linux-x86_64-w1/archbench",
        "bin/linux-x86_64-w1/tactics",
        "archbench/constructed-2/*",
        "archbench/constructed-ood-1/*",
        "archbench/tactics-4/*",
        "archbench/pro-decisions-1/*",
        "Mage.Server/career-src/*",
        "code/rl/infer_server.py", "code/rl/model.py", "code/rl/cardtext.py", "code/rl/train.py", "code/rl/data.py",
        "code/rl-w1/*",
        "cardtext/*",
    ] + [f"checkpoints/{c}/*" for c in ckpts]
    snapshot_download(
        repo_id="nerijs/playmat-ai-checkpoints",
        repo_type="model",
        local_dir=work,
        allow_patterns=patterns,
        token=tok,
    )

    for sub in ("rl", "rl-w1"):
        rdir = os.path.join(work, "code", sub)
        os.makedirs(os.path.join(rdir, "data"), exist_ok=True)
        for fn in ("cardtext.npy", "cardtext_sem.npy"):
            src, dst = os.path.join(work, "cardtext", fn), os.path.join(rdir, "data", fn)
            if os.path.exists(src) and not os.path.exists(dst):
                os.symlink(src, dst)
        # Ensure cardtext.py is present in rl-w1 too
        ct_src = os.path.join(work, "code", "rl", "cardtext.py")
        ct_dst = os.path.join(rdir, "cardtext.py")
        if os.path.exists(ct_src) and not os.path.exists(ct_dst):
            os.symlink(ct_src, ct_dst)

    old_ab = os.path.join(work, "bin", "linux-x86_64", "archbench")
    new_ab = os.path.join(work, "bin", "linux-x86_64-w1", "archbench")
    new_tac = os.path.join(work, "bin", "linux-x86_64-w1", "tactics")
    for b in (old_ab, new_ab, new_tac):
        os.chmod(b, 0o755)

    env = dict(os.environ, PLAYMAT_GAME_TIMEOUT_SECS="1200", PLAYMAT_REPO=work)
    flags_256 = (
        "+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000"
        "+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        "+extra_turns=2+belief_ms=1+fpu_vnode=1+solver=1+vloss=16+threat_w=500+playout=2"
    )
    flags_1024 = (
        "+hide=1+iters=1024+samples=8+cpuct=1000+leaf_blend=1000"
        "+gumbel_root=1+multi_tree=1+mulligan_net=1+width=24+consider=16"
        "+extra_turns=2+belief_ms=1+fpu_vnode=1+solver=1+vloss=16+threat_w=500+playout=2"
    )

    # ------------------------------------------------------------------
    # PART 1: SPEEDUP A/B (OLD Phase 6 Stack vs NEW Wave 1 Stack, same GPU)
    # ------------------------------------------------------------------
    print("\n================ PART 1A: OLD STACK BASELINE (iters=256, 2-way, deals=%d) ================" % args.deals_speed, flush=True)
    old_lf = open(os.path.join(work, "old_srv.log"), "w")
    old_srv = subprocess.Popen(
        [sys.executable, "-u", os.path.join(work, "code", "rl", "infer_server.py"),
         "--dir", os.path.join(work, "checkpoints", "phase6-final-10m-v4-soup"),
         "--port", "18810", "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5"],
        cwd=os.path.join(work, "code", "rl"), stdout=old_lf, stderr=subprocess.STDOUT,
    )
    try:
        assert wait_port("127.0.0.1", 18810, 120.0), "old infer_server failed"
        old_e = f"cfg:policy=ismcts+net=tcp://127.0.0.1:18810{flags_256}"
        dt_old_256, _ = run_cmd(
            [old_ab, "--suite", os.path.join(work, "archbench", "constructed-2"),
             "--entrants", f"{old_e},host@s6", "--deals", str(args.deals_speed), "--jobs", str(args.jobs)],
            work, env,
        )
        old_e1024 = f"cfg:policy=ismcts+net=tcp://127.0.0.1:18810{flags_1024}"
        dt_old_1024, _ = run_cmd(
            [old_ab, "--suite", os.path.join(work, "archbench", "constructed-2"),
             "--entrants", f"{old_e1024},host@s6", "--deals", "2", "--jobs", "12"],
            work, env,
        )
    finally:
        old_srv.terminate()
        try: old_srv.wait(timeout=5)
        except Exception: old_srv.kill()
        old_lf.close()

    print("\n================ PART 1B: NEW WAVE 1 STACK (single multi-model unix:// server) ================", flush=True)
    sock_10m = "/tmp/pm_w1_10m.sock"
    sock_5m = "/tmp/pm_w1_5m.sock"
    sock_2m = "/tmp/pm_w1_2m.sock"
    for sp in (sock_10m, sock_5m, sock_2m):
        if os.path.exists(sp):
            os.unlink(sp)
    models_spec = ",".join([
        f"unix://{sock_10m}={os.path.join(work, 'checkpoints', 'phase6-final-10m-v4-soup')}",
        f"unix://{sock_5m}={os.path.join(work, 'checkpoints', 'p6pre-5m-carddrop')}",
        f"unix://{sock_2m}={os.path.join(work, 'checkpoints', 'phase6-final-1.3m-v4-soup')}",
    ])
    new_lf = open(os.path.join(work, "new_srv.log"), "w")
    srv_cmd = [
        sys.executable, "-u", os.path.join(work, "code", "rl-w1", "infer_server.py"),
        "--models", models_spec, "--amp", "1", "--max-batch", "512", "--wait-ms", "0.5",
    ]
    if args.compile:
        srv_cmd += ["--compile"]
    new_srv = subprocess.Popen(
        srv_cmd, cwd=os.path.join(work, "code", "rl-w1"), stdout=new_lf, stderr=subprocess.STDOUT,
    )
    try:
        for sp in (sock_10m, sock_5m, sock_2m):
            assert wait_unix(sp, 180.0), f"new infer_server socket missing: {sp}"
        print("NEW MULTI-MODEL UNIX INFER SERVER READY", flush=True)

        # Warmup 1 deal if compiled
        new_e = f"cfg:policy=ismcts+net=unix://{sock_10m}{flags_256}"
        if args.compile:
            run_cmd(
                [new_ab, "--suite", os.path.join(work, "archbench", "constructed-2"),
                 "--entrants", f"{new_e},host@s6", "--deals", "1", "--jobs", "4"],
                work, env,
            )

        dt_new_256, _ = run_cmd(
            [new_ab, "--suite", os.path.join(work, "archbench", "constructed-2"),
             "--entrants", f"{new_e},host@s6", "--deals", str(args.deals_speed), "--jobs", str(args.jobs)],
            work, env,
        )
        new_e1024 = f"cfg:policy=ismcts+net=unix://{sock_10m}{flags_1024}"
        dt_new_1024, _ = run_cmd(
            [new_ab, "--suite", os.path.join(work, "archbench", "constructed-2"),
             "--entrants", f"{new_e1024},host@s6", "--deals", "2", "--jobs", "12"],
            work, env,
        )
        print(
            f"\nSPEEDUP SUMMARY ({dev_name}): "
            f"iters=256 OLD={dt_old_256:.1f}s NEW={dt_new_256:.1f}s ({dt_old_256/max(dt_new_256,0.1):.2f}x) | "
            f"iters=1024 OLD={dt_old_1024:.1f}s NEW={dt_new_1024:.1f}s ({dt_old_1024/max(dt_new_1024,0.1):.2f}x)",
            flush=True,
        )

        if not args.skip_elo_gate:
            # ------------------------------------------------------------------
            # PART 2: WAVE 1 SEARCH ELO GATE (240 FAIR GAMES: 120 C2 + 120 OOD)
            # ------------------------------------------------------------------
            print(f"\n================ PART 2: WAVE 1 FAIR ELO GATE (deals={args.deals_ab}, 4-way = {args.deals_ab*12}g/suite) ================", flush=True)
            e_10m = f"cfg:policy=ismcts+net=unix://{sock_10m}{flags_256}"
            e_5m = f"cfg:policy=ismcts+net=unix://{sock_5m}{flags_256}"
            e_5m_pw = f"cfg:policy=ismcts+net=unix://{sock_5m}{flags_256}+prog_worlds=1"
            print("LEGEND: 10m=phase6-final-10m-v4-soup | 5m=p6pre-5m-carddrop | 5m_pw=p6pre-5m-carddrop+prog_worlds=1 | host@s6", flush=True)
            for s in ("constructed-2", "constructed-ood-1"):
                print(f"--- WAVE 1 GATE SUITE: {s} ---", flush=True)
                run_cmd(
                    [new_ab, "--suite", os.path.join(work, "archbench", s),
                     "--entrants", f"{e_10m},{e_5m},{e_5m_pw},host@s6",
                     "--deals", str(args.deals_ab), "--jobs", str(args.jobs)],
                    work, env,
                )

            # ------------------------------------------------------------------
            # PART 3: WAVE 1 PUZZLES (tactics@4 hide=0 & hide=1, pro-decisions@1 hide=1)
            # ------------------------------------------------------------------
            print("\n================ PART 3: WAVE 1 PUZZLE CHECK ================", flush=True)
            p2m_f = f"cfg:policy=ismcts+net=unix://{sock_2m}{flags_256}"
            p2m_o = p2m_f.replace("+hide=1", "+hide=0")
            p5m_f = f"cfg:policy=ismcts+net=unix://{sock_5m}{flags_256}"
            run_cmd([new_tac, "--suite", os.path.join(work, "archbench", "tactics-4"), "--entrants", f"{p2m_o},{p2m_f},{p5m_f}"], work, env)
            run_cmd([new_tac, "--suite", os.path.join(work, "archbench", "pro-decisions-1"), "--entrants", f"{p2m_f},{p5m_f}"], work, env)
    finally:
        new_srv.terminate()
        try: new_srv.wait(timeout=5)
        except Exception: new_srv.kill()
        new_lf.close()
        try:
            print("NEW_SRV_TAIL:\n" + "\n".join(open(os.path.join(work, "new_srv.log")).read().splitlines()[-15:]), flush=True)
        except Exception:
            pass

    # ------------------------------------------------------------------
    # PART 4: TRAINING PIPELINE COMPILE + THROUGHPUT SANITY CHECK (1 shard)
    # ------------------------------------------------------------------
    print("\n================ PART 4: WAVE 1 TRAIN.PY COMPILE + HL-GAUSS + CACHE-ORACLE CHECK ================", flush=True)
    shard_dir = os.path.join(work, "shards")
    snapshot_download(
        repo_id="nerijs/playmat-ai-selfplay",
        repo_type="dataset",
        local_dir=shard_dir,
        allow_patterns=["data/phase6-r26a-5m-train450-colab/merged.*"],
        token=tok,
    )
    out_ckpt = os.path.join(work, "tmp_train_smoke")
    train_cmd = [
        sys.executable, "-u", os.path.join(work, "code", "rl-w1", "train.py"),
        "--data", os.path.join(shard_dir, "data", "phase6-r26a-5m-train450-colab"),
        "--out", out_ckpt,
        "--arch", "tx", "--tx", "d=256,layers=6,heads=8,ff=512",
        "--epochs", "2", "--bs", "1024", "--lr", "3e-4", "--opt", "muon",
        "--upgrade-val-bins", "51", "--val-prob", "0.50", "--card-id-drop", "0.50",
        "--threat-weight", "0.15", "--act-q-weight", "0.15", "--aux-weight", "0.15",
        "--compile",
    ]
    dt_tr, _ = run_cmd(train_cmd, os.path.join(work, "code", "rl-w1"), env)
    print(f"WAVE1_TRAIN_SMOKE_OK elapsed={dt_tr:.1f}s total_gate={time.time()-t_start:.1f}s", flush=True)


if __name__ == "__main__":
    main()

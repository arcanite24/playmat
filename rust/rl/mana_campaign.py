#!/usr/bin/env python3
"""Unattended Mana@Home flagship training campaign (`mana-zero`).

Runs an independent, from-scratch community Transformer (`Tx`) lineage on https://mana.deimos.studio,
completely separate from GCP/Colab checkpoints while adopting the Phase 4-6 architecture and training
lessons:
- Obs-v4 (`GLOB_F=97`) Transformer (`d=192, heads=6, layers=4, ff=384, dim=64, trunk=192, gelu=1, rel=1,
  act_ent=1, val_mlp=1, val_bins=21`) with ThreatHead + OptPol + ActQ + Muon optimizer + EMA.
- Disjoint 220-deck training suite (`archbench/constructed-train-200`) with held-out `constructed-2`
  SPRT promotion gates and Archbench Elo ratings.
- Dual-track CPU (`strict-1`) + GPU (`selfplay-gpu`, `vloss=8`) units per round so all volunteers stay
  busy with KataGo playout-cap randomization and Phase 4/5/6 search (`fpu_vnode=1+solver=1+forced_k=200`).
- Self-recovering `state.json` state machine + automatic R2 intake drain (`purge-intake`, optional HF
  dataset archiving) after each round is trained so the campaign runs unattended indefinitely.
"""

import argparse
import json
import os
import pathlib
import subprocess
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../.."))
sys.path.insert(0, HERE)

import model as model_mod  # noqa: E402
import swarm  # noqa: E402

DEFAULT_TX_CFG = {
    "d": 192,
    "heads": 6,
    "layers": 4,
    "ff": 384,
    "dim": 64,
    "trunk": 192,
    "gelu": 1,
    "rel": 1,
    "act_ent": 1,
    "val_mlp": 1,
    "val_bins": 21,
    "glob_f": model_mod.GLOB_F,
    "threat_head": 1,
    "text": 0,
}

SEARCH_COMMON = (
    "+hide=1+samples=6+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1"
    "+mulligan_net=1+width=20+consider=14+extra_turns=2+belief_ms=1"
    "+fpu_vnode=1+solver=1+forced_k=200"
)


def log(msg):
    ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    print(f"[{ts}] CAMPAIGN {msg}", flush=True)


def save_state(path, state):
    tmp = str(path) + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=2, sort_keys=True)
    os.replace(tmp, path)


def load_state(path):
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    return None


def init_gen0(work_dir, project, run_name, tx_cfg, seed=20261127):
    import torch

    g0_dir = os.path.join(work_dir, "checkpoints", "g0")
    os.makedirs(g0_dir, exist_ok=True)
    pmnet_path = os.path.join(g0_dir, "net.pmnet")
    pt_path = os.path.join(g0_dir, "net.pt")
    arch_path = os.path.join(g0_dir, "arch.json")

    if os.path.exists(pmnet_path) and os.path.exists(pt_path) and os.path.exists(arch_path):
        return g0_dir, pmnet_path

    torch.manual_seed(seed)
    net = model_mod.Tx(**tx_cfg)
    model_mod.save_dir(net, g0_dir)
    model_mod.export(net, pmnet_path, {"project": project, "run": run_name, "generation": 0, "kind": "random-init"})
    log(f"Initialized fresh Gen 0 Tx ({model_mod.params(net):,} params) -> {pmnet_path}")
    return g0_dir, pmnet_path


def make_selfplay_round_units(
    client,
    project,
    run_name,
    rnd,
    best_pmnet,
    ancestors,
    train_suite_dir,
    cpu_units,
    gpu_units,
    unit_games,
    iters,
    fast_iters,
    keep,
    vloss,
    seed_base,
    obs=4,
):
    nets = {}
    cpu_learner = swarm.swarm_spec(
        f"cfg:policy=ismcts+net={best_pmnet}+iters={iters}{SEARCH_COMMON}+temp=1000+temp_turn=6",
        client,
        nets,
    )
    gpu_learner = swarm.swarm_spec(
        f"cfg:policy=ismcts+net={best_pmnet}+iters={iters}{SEARCH_COMMON}+vloss={vloss}+temp=1000+temp_turn=6",
        client,
        nets,
    )

    opp_iters = max(64, iters // 2)
    cpu_opps = [{"config": "self", "weight": 4}]
    gpu_opps = [{"config": "self", "weight": 4}]
    if not ancestors:
        cpu_opps.append({"config": "host@s6", "weight": 2})
        cpu_opps.append({"config": "cfg:policy=quick", "weight": 1})
        gpu_opps.append({"config": "host@s6", "weight": 2})
        gpu_opps.append({"config": "cfg:policy=quick", "weight": 1})
    else:
        for anc_path in ancestors[-2:]:
            if os.path.exists(anc_path):
                anc_cpu = swarm.swarm_spec(
                    f"cfg:policy=ismcts+net={anc_path}+iters={opp_iters}{SEARCH_COMMON}+temp=750+temp_turn=6",
                    client,
                    nets,
                )
                anc_gpu = swarm.swarm_spec(
                    f"cfg:policy=ismcts+net={anc_path}+iters={opp_iters}{SEARCH_COMMON}+vloss={vloss}+temp=750+temp_turn=6",
                    client,
                    nets,
                )
                cpu_opps.append({"config": anc_cpu, "weight": 1})
                gpu_opps.append({"config": anc_gpu, "weight": 1})
        cpu_opps.append({"config": "host@s6", "weight": 1})
        gpu_opps.append({"config": "host@s6", "weight": 1})

    cpu_refs = swarm.net_refs(client, cpu_learner, [o["config"] for o in cpu_opps], nets)
    gpu_refs = swarm.net_refs(client, gpu_learner, [o["config"] for o in gpu_opps], nets)
    suite, suite_sha = swarm.suite_pin(train_suite_dir)

    units = []
    deal_cursor = seed_base
    fast_spec = f"ismcts:{fast_iters}" if fast_iters > 0 else None

    for i in range(cpu_units):
        units.append({
            "protocol": swarm.PROTOCOL,
            "id": f"{project}-{run_name}-r{rnd}-cpu{i:03d}",
            "project": project,
            "run": run_name,
            "round": rnd,
            "kind": "selfplay",
            "pins": {"builds": [], "obs": obs, "suite": suite, "suite_sha256": suite_sha, "strict": True},
            "nets": cpu_refs,
            "learner": cpu_learner,
            "opponents": cpu_opps,
            "seed": deal_cursor,
            "games": unit_games,
            "keep": keep,
            "fast": fast_spec,
            "deadline": 0,
            "cost": 1.0,
        })
        deal_cursor += unit_games

    for i in range(gpu_units):
        units.append({
            "protocol": swarm.PROTOCOL,
            "id": f"{project}-{run_name}-r{rnd}-gpu{i:03d}",
            "project": project,
            "run": run_name,
            "round": rnd,
            "kind": "selfplay-gpu",
            "pins": {"builds": [], "obs": obs, "suite": suite, "suite_sha256": suite_sha, "strict": False},
            "nets": gpu_refs,
            "learner": gpu_learner,
            "opponents": gpu_opps,
            "seed": deal_cursor,
            "games": unit_games,
            "keep": keep,
            "fast": fast_spec,
            "deadline": 0,
            "cost": 1.0,
        })
        deal_cursor += unit_games

    return units


def publish_eval_bench(client, project, run_name, gen, pmnet_path, label, eval_suite_dir, games, unit_games, iters, seed_base, obs=4):
    nets = {}
    learner = swarm.swarm_spec(
        f"cfg:policy=ismcts+net={pmnet_path}+iters={iters}{SEARCH_COMMON}+temp=0",
        client,
        nets,
    )
    sha = list(nets.values())[0]
    refs = swarm.net_refs(client, learner, ["host@s6"], nets)
    suite, suite_sha = swarm.suite_pin(eval_suite_dir)
    units = []
    for i, start in enumerate(range(0, games, unit_games)):
        units.append({
            "protocol": swarm.PROTOCOL,
            "id": f"{project}-bench-g{gen}-u{i:03d}",
            "project": f"{project}-bench",
            "run": run_name,
            "round": gen,
            "kind": "bench",
            "pins": {"builds": [], "obs": obs, "suite": suite, "suite_sha256": suite_sha, "strict": True},
            "nets": refs,
            "learner": learner,
            "opponents": [{"config": "host@s6", "weight": 1}],
            "seed": seed_base + start,
            "games": min(unit_games, games - start),
            "keep": 0,
            "fast": None,
            "deadline": 0,
            "cost": 1.0,
        })
    return client.call("POST", "/bench", {
        "project": f"{project}-bench",
        "run": run_name,
        "round": gen,
        "priority": 30,
        "net_sha": sha,
        "net_label": label,
        "anchor_elo": 1000.0,
        "units": units,
    })


def publish_sprt_gate(
    client,
    project,
    run_name,
    gen,
    cand_pmnet,
    cand_label,
    base_pmnet,
    base_label,
    eval_suite_dir,
    games,
    unit_games,
    iters,
    seed_base,
    elo0=0.0,
    elo1=35.0,
    max_share=1.0,
    obs=4,
):
    nets = {}
    learner = swarm.swarm_spec(
        f"cfg:policy=ismcts+net={cand_pmnet}+iters={iters}{SEARCH_COMMON}+temp=0",
        client,
        nets,
    )
    opponent = swarm.swarm_spec(
        f"cfg:policy=ismcts+net={base_pmnet}+iters={iters}{SEARCH_COMMON}+temp=0",
        client,
        nets,
    )
    cand_sha = swarm.sha256_file(cand_pmnet)
    base_sha = swarm.sha256_file(base_pmnet)
    refs = swarm.net_refs(client, learner, [opponent], nets)
    suite, suite_sha = swarm.suite_pin(eval_suite_dir)
    units = []
    for i, start in enumerate(range(0, games, unit_games)):
        units.append({
            "protocol": swarm.PROTOCOL,
            "id": f"{project}-gate-g{gen}-u{i:03d}",
            "project": f"{project}-gate",
            "run": run_name,
            "round": gen,
            "kind": "match",
            "pins": {"builds": [], "obs": obs, "suite": suite, "suite_sha256": suite_sha, "strict": True},
            "nets": refs,
            "learner": learner,
            "opponents": [{"config": opponent, "weight": 1}],
            "seed": seed_base + start,
            "games": min(unit_games, games - start),
            "keep": 0,
            "fast": None,
            "deadline": 0,
            "cost": 1.0,
        })
    return client.call("POST", "/gates", {
        "project": f"{project}-gate",
        "run": run_name,
        "round": gen,
        "priority": 35,
        "candidate_sha": cand_sha,
        "candidate_label": cand_label,
        "baseline_sha": base_sha,
        "baseline_label": base_label,
        "elo0": elo0,
        "elo1": elo1,
        "alpha": 0.05,
        "beta": 0.05,
        "max_share": max_share,
        "auto_promote": True,
        "units": units,
    })


def train_candidate(args, init_dir, data_dirs, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    cmd = [
        sys.executable,
        "-u",
        os.path.join(HERE, "train.py"),
        "--arch", "tx",
        "--init", init_dir,
        "--out", out_dir,
        "--epochs", str(args.epochs),
        "--bs", str(args.bs),
        "--opt", "muon",
        "--muon-lr", str(args.muon_lr),
        "--lr", str(args.lr),
        "--drop", "0.10",
        "--no-critic",
        "--val-prob", str(args.val_prob),
        "--val-wd", "0.05",
        "--q-mix", "0.25",
        "--td-lambda", "0.5",
        "--aux-weight", "0.20",
        "--act-q-weight", "0.50",
        "--threat-weight", "0.15",
        "--opt-pol-weight", "0.20",
        "--recency-decay", "0.93",
        "--policy-recent", str(min(len(data_dirs), 8)),
        "--pol-old-weight", "0.25",
        "--ema", "0.995",
        "--gpu-data-gb", str(args.gpu_data_gb),
        "--stage-chunk", str(args.stage_chunk),
        "--compact",
    ]
    for d in data_dirs:
        cmd += ["--data", d]
    log(f"Running local training -> {out_dir} ({len(data_dirs)} replay rounds)")
    subprocess.run(cmd, cwd=HERE, check=True)
    pmnet = os.path.join(out_dir, "net.pmnet")
    if not os.path.exists(pmnet):
        raise RuntimeError(f"train.py did not produce {pmnet}")
    return pmnet


def wait_gate(client, gate_id, poll=30):
    while True:
        g = client.call("GET", f"/gates/{gate_id}")
        st = g["status"]
        log(f"Gate #{gate_id} ({g['candidate_label']}): status={st} W/D/L={g['wins']}/{g['draws']}/{g['losses']} elo={g['elo']:+.1f} llr={g['llr']:.2f}")
        if st in ("passed", "failed", "promoted", "rejected"):
            return g
        rd = client.call("GET", f"/rounds/{g['round_id']}")
        u = rd["units"]
        if not u.get("queued") and not u.get("leased") and rd.get("verify_pending", 0) == 0:
            return client.call("GET", f"/gates/{gate_id}")
        time.sleep(poll)


def main():
    ap = argparse.ArgumentParser(description="Unattended Mana@Home flagship training campaign (mana-zero)")
    ap.add_argument("--coordinator", default=os.environ.get("MANA_COORDINATOR", "https://mana.deimos.studio"))
    ap.add_argument("--token-file", default=os.environ.get("MANA_TOKEN_FILE", os.path.expanduser("~/.config/mana/trainer.token")))
    ap.add_argument("--work-dir", required=True, help="Isolated campaign directory (checkpoints, replay data, state.json)")
    ap.add_argument("--repo", default=REPO)
    ap.add_argument("--project", default="mana-zero")
    ap.add_argument("--run", default="tx-v1")
    ap.add_argument("--train-suite", default="constructed-train-450", help="Training deck suite under archbench/")
    ap.add_argument("--eval-suite", default="constructed-2", help="Held-out evaluation suite under archbench/")
    ap.add_argument("--cpu-units", type=int, default=8, help="CPU strict-1 units per round")
    ap.add_argument("--gpu-units", type=int, default=4, help="GPU selfplay-gpu units per round")
    ap.add_argument("--unit-games", type=int, default=25, help="Games per self-play unit")
    ap.add_argument("--iters", type=int, default=128, help="ISMCTS iterations per full-search move")
    ap.add_argument("--fast-iters", type=int, default=32, help="KataGo fast-search iterations for non-recorded moves")
    ap.add_argument("--keep", type=int, default=400, help="KataGo playout-cap keep rate per 1000")
    ap.add_argument("--vloss", type=int, default=8, help="Virtual loss batching for GPU units")
    ap.add_argument("--eval-iters", type=int, default=96, help="ISMCTS iterations for Archbench and SPRT gates")
    ap.add_argument("--bench-games", type=int, default=40)
    ap.add_argument("--gate-games", type=int, default=120)
    ap.add_argument("--gate-max-share", type=float, default=1.0, help="Single-account concentration cap for SPRT auto-promotion")
    ap.add_argument("--replay-window", type=int, default=12, help="Max recent accepted rounds kept in local replay buffer")
    ap.add_argument("--epochs", type=int, default=4)
    ap.add_argument("--bs", type=int, default=1024)
    ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--muon-lr", type=float, default=0.004)
    ap.add_argument("--val-prob", type=float, default=0.20)
    ap.add_argument("--gpu-data-gb", type=float, default=6.0)
    ap.add_argument("--stage-chunk", type=int, default=600_000)
    ap.add_argument("--poll", type=int, default=30)
    ap.add_argument("--hf-dataset", default="", help="Optional HF dataset repo to archive accepted rounds before purging R2")
    ap.add_argument("--no-purge-r2", action="store_true", help="Keep R2 intake objects after training (default purges to bound R2 storage)")
    ap.add_argument("--max-rounds", type=int, default=0, help="Stop after N rounds (0 = run forever)")
    args = ap.parse_args()

    os.makedirs(args.work_dir, exist_ok=True)
    state_path = os.path.join(args.work_dir, "state.json")
    token = pathlib.Path(args.token_file).read_text(encoding="utf-8").strip()
    client = swarm.Client(args.coordinator, token)

    train_suite = os.path.join(args.repo, "archbench", args.train_suite)
    if not os.path.exists(os.path.join(train_suite, "suite.json")):
        train_suite = os.path.join(args.repo, "archbench", "constructed-train-200")
    eval_suite = os.path.join(args.repo, "archbench", args.eval_suite)

    state = load_state(state_path)
    if state is None:
        g0_dir, g0_pmnet = init_gen0(args.work_dir, args.project, args.run, DEFAULT_TX_CFG)
        g0_sha = swarm.sha256_file(g0_pmnet)
        # Rate Gen 0 baseline on held-out constructed-2 so /swarm/networks starts from Gen 0
        bench0 = publish_eval_bench(
            client,
            args.project,
            args.run,
            0,
            g0_pmnet,
            f"{args.project}-g0",
            eval_suite,
            games=args.bench_games,
            unit_games=20,
            iters=args.eval_iters,
            seed_base=8_000_000,
        )
        log(f"Published Gen 0 Archbench baseline (bench #{bench0['id']}, round #{bench0['round_id']})")
        state = {
            "project": args.project,
            "run": args.run,
            "gen": 1,
            "best_gen": 0,
            "best_dir": g0_dir,
            "best_pmnet": g0_pmnet,
            "best_sha": g0_sha,
            "ancestors": [],
            "data_dirs": [],
            "phase": "publish_selfplay",
            "active_round_id": None,
            "active_gate_id": None,
            "active_bench_round_id": bench0["round_id"],
            "cand_dir": None,
            "cand_pmnet": None,
            "rounds_completed": 0,
        }
        save_state(state_path, state)

    while True:
        if args.max_rounds > 0 and state["rounds_completed"] >= args.max_rounds:
            log(f"Reached --max-rounds={args.max_rounds}; exiting cleanly.")
            return

        try:
            _step(args, client, state, state_path, train_suite, eval_suite)
        except (KeyboardInterrupt, SystemExit):
            raise
        except Exception as exc:
            log(f"Transient error in phase={state.get('phase')}: {exc}")
            traceback.print_exc()
            log("Sleeping 60s before retrying current phase...")
            time.sleep(60)


def _step(args, client, state, state_path, train_suite, eval_suite):
    gen = state["gen"]
    phase = state["phase"]

    if phase == "publish_selfplay":
        seed_base = 10_000_000 + gen * 100_000
        units = make_selfplay_round_units(
            client,
            args.project,
            args.run,
            gen,
            state["best_pmnet"],
            state["ancestors"],
            train_suite,
            cpu_units=args.cpu_units,
            gpu_units=args.gpu_units,
            unit_games=args.unit_games,
            iters=args.iters,
            fast_iters=args.fast_iters,
            keep=args.keep,
            vloss=args.vloss,
            seed_base=seed_base,
        )
        st = swarm.publish(client, args.project, args.run, gen, units, priority=15)
        state["active_round_id"] = st["id"]
        state["phase"] = "wait_selfplay"
        save_state(state_path, state)
        log(f"Published Gen {gen} self-play round #{st['id']} ({args.cpu_units} CPU + {args.gpu_units} GPU units x {args.unit_games} games)")

    elif phase == "wait_selfplay":
        rid = state["active_round_id"]
        swarm.wait(client, rid, poll=args.poll, wait_verify=True)
        out_dir = os.path.join(args.work_dir, "data", f"r{gen:03d}")
        ms = swarm.pull_accepted(client, rid, out_dir)
        if not ms:
            log(f"Round #{rid} finished with 0 accepted units; retrying next generation number")
            state["gen"] = gen + 1
            state["phase"] = "publish_selfplay"
            save_state(state_path, state)
            return
        log(f"Pulled {len(ms)} accepted units ({sum(len(m['games']) for m in ms.values())} games) into {out_dir}")
        if out_dir not in state["data_dirs"]:
            state["data_dirs"].append(out_dir)
        state["data_dirs"] = state["data_dirs"][-args.replay_window :]
        state["phase"] = "train"
        save_state(state_path, state)

    elif phase == "train":
        cand_dir = os.path.join(args.work_dir, "checkpoints", f"g{gen}")
        cand_pmnet = train_candidate(args, state["best_dir"], state["data_dirs"], cand_dir)
        rid = state["active_round_id"]
        client.call("POST", f"/admin/rounds/{rid}/trained", {})
        archive_ref = ""
        if args.hf_dataset:
            try:
                import hf_archive

                api = hf_archive.HfApi(token=hf_archive.load_token())
                hf_archive.ensure_repo(api, args.hf_dataset, "dataset")
                remote = f"{args.project}-{args.run}-r{gen:03d}"
                hf_archive.upload_data_dir(api, args.hf_dataset, state["data_dirs"][-1], remote_name=remote, compact=True)
                archive_ref = f"hf://datasets/{args.hf_dataset}/data/{remote}"
            except Exception as exc:
                log(f"HF archive skipped (non-fatal): {exc}")
        if not args.no_purge_r2:
            purged = client.call("POST", f"/admin/rounds/{rid}/purge-intake", {
                "force": False,
                "include_rejected": True,
                "archive_ref": archive_ref or f"local:{state['data_dirs'][-1]}",
            })
            log(f"Purged R2 intake for round #{rid}: {purged}")

        state["cand_dir"] = cand_dir
        state["cand_pmnet"] = cand_pmnet
        state["phase"] = "publish_eval"
        save_state(state_path, state)

    elif phase == "publish_eval":
        cand_label = f"{args.project}-g{gen}"
        base_label = f"{args.project}-g{state['best_gen']}"
        bench = publish_eval_bench(
            client,
            args.project,
            args.run,
            gen,
            state["cand_pmnet"],
            cand_label,
            eval_suite,
            games=args.bench_games,
            unit_games=20,
            iters=args.eval_iters,
            seed_base=8_000_000 + gen * 1000,
        )
        gate = publish_sprt_gate(
            client,
            args.project,
            args.run,
            gen,
            state["cand_pmnet"],
            cand_label,
            state["best_pmnet"],
            base_label,
            eval_suite,
            games=args.gate_games,
            unit_games=20,
            iters=args.eval_iters,
            seed_base=9_000_000 + gen * 1000,
            max_share=args.gate_max_share,
        )
        log(f"Published eval for {cand_label}: bench #{bench['id']} + SPRT gate #{gate['id']} vs {base_label}")
        state["active_bench_round_id"] = bench["round_id"]
        state["active_gate_id"] = gate["id"]
        state["phase"] = "wait_eval"
        save_state(state_path, state)

    elif phase == "wait_eval":
        gate_res = wait_gate(client, state["active_gate_id"], poll=args.poll)
        if state.get("active_bench_round_id"):
            swarm.wait(client, state["active_bench_round_id"], poll=args.poll, wait_verify=True)
        if gate_res["status"] in ("passed", "promoted"):
            log(f"PROMOTED {gate_res['candidate_label']} (elo {gate_res['elo']:+.1f}, llr {gate_res['llr']:.2f})")
            if state["best_pmnet"] not in state["ancestors"]:
                state["ancestors"].append(state["best_pmnet"])
            state["ancestors"] = state["ancestors"][-6:]
            state["best_gen"] = gen
            state["best_dir"] = state["cand_dir"]
            state["best_pmnet"] = state["cand_pmnet"]
            state["best_sha"] = swarm.sha256_file(state["cand_pmnet"])
        else:
            log(f"KEPT {args.project}-g{state['best_gen']} (candidate {gate_res['candidate_label']} status={gate_res['status']} elo={gate_res['elo']:+.1f})")
        state["gen"] = gen + 1
        state["rounds_completed"] += 1
        state["active_round_id"] = None
        state["active_gate_id"] = None
        state["active_bench_round_id"] = None
        state["cand_dir"] = None
        state["cand_pmnet"] = None
        state["phase"] = "publish_selfplay"
        save_state(state_path, state)


if __name__ == "__main__":
    main()

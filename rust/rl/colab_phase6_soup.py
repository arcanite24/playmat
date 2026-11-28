"""Build a uniform Model Soup on Colab/CPU from HF checkpoints and upload back to HF."""
import argparse, json, os, subprocess, sys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-name", required=True)
    ap.add_argument("--members", nargs="+", required=True)
    ap.add_argument("--models-repo", default="nerijs/playmat-ai-checkpoints")
    args = ap.parse_args()

    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"])
    import torch
    from huggingface_hub import HfApi, snapshot_download

    tok = os.environ["HF_TOKEN"]
    work = "/content/soup_p6"
    os.makedirs(work, exist_ok=True)
    pats = ["code/rl/model.py", "code/rl/cardtext.py", "cardtext/*"]
    for c in args.members:
        pats += [f"checkpoints/{c}/net.pt", f"checkpoints/{c}/arch.json"]
    snapshot_download(repo_id=args.models_repo, repo_type="model", local_dir=work, allow_patterns=pats, token=tok)

    rl_dir = os.path.join(work, "code", "rl")
    os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
    for fn in ("cardtext.npy", "cardtext_sem.npy"):
        src, dst = os.path.join(work, "cardtext", fn), os.path.join(rl_dir, "data", fn)
        if os.path.exists(src) and not os.path.exists(dst):
            os.symlink(src, dst)
    sys.path.insert(0, rl_dir)
    os.chdir(rl_dir)
    import model

    print(f"=== BUILDING PHASE 6 SOUP: {args.out_name} from {args.members} ===", flush=True)
    sds, arch = [], None
    for c in args.members:
        d = os.path.join(work, "checkpoints", c)
        if arch is None:
            arch = json.load(open(os.path.join(d, "arch.json")))
        sds.append(torch.load(os.path.join(d, "net.pt"), map_location="cpu", weights_only=True))
    avg = {}
    for k in sds[0].keys():
        if sds[0][k].is_floating_point():
            avg[k] = (sum(sd[k].float() for sd in sds) / len(sds)).to(sds[0][k].dtype)
        else:
            avg[k] = sds[0][k]

    out_dir = os.path.join(work, "out", args.out_name)
    os.makedirs(out_dir, exist_ok=True)
    torch.save(avg, os.path.join(out_dir, "net.pt"))
    with open(os.path.join(out_dir, "arch.json"), "w") as f:
        json.dump(arch, f, indent=2)
    net = model.load_dir(out_dir, torch.device("cpu")).eval()
    model.export(net, os.path.join(out_dir, "net.pmnet"))
    pm_bytes = os.path.getsize(os.path.join(out_dir, "net.pmnet"))
    has_threat = getattr(net, "threat", None) is not None
    with open(os.path.join(out_dir, "report.json"), "w") as f:
        json.dump({"soup_members": args.members, "arch": arch, "pmnet_bytes": pm_bytes, "has_threat": has_threat}, f, indent=2)

    api = HfApi(token=tok)
    api.upload_folder(
        folder_path=out_dir,
        path_in_repo=f"checkpoints/{args.out_name}",
        repo_id=args.models_repo,
        repo_type="model",
        commit_message=f"Phase 6 Model Soup: {args.out_name} ({len(args.members)} members)",
    )
    print(f"SOUP_OK {args.out_name} pmnet_bytes={pm_bytes} has_threat={has_threat}", flush=True)


if __name__ == "__main__":
    main()

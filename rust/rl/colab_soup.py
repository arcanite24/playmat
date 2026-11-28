
import json, os, shutil, sys
subprocess_cmd = [sys.executable, "-m", "pip", "install", "-q", "huggingface_hub", "hf_xet"]
import subprocess
subprocess.check_call(subprocess_cmd)
import torch
from huggingface_hub import HfApi, snapshot_download

tok = os.environ["HF_TOKEN"]
work = "/content/soup_work"
os.makedirs(work, exist_ok=True)
snapshot_download(
    repo_id="nerijs/playmat-ai-checkpoints",
    repo_type="model",
    local_dir=work,
    allow_patterns=["code/rl/model.py", "code/rl/cardtext.py", "cardtext/*"],
    token=tok,
)
rl_dir = os.path.join(work, "code", "rl")
os.makedirs(os.path.join(rl_dir, "data"), exist_ok=True)
for fn in ("cardtext.npy", "cardtext_sem.npy"):
    src = os.path.join(work, "cardtext", fn)
    dst = os.path.join(rl_dir, "data", fn)
    if os.path.exists(src) and not os.path.exists(dst):
        os.symlink(src, dst)
sys.path.insert(0, rl_dir)
os.chdir(rl_dir)
import model

def make_soup(out_name, ckpts):
    print(f"=== BUILDING MODEL SOUP: {out_name} from {ckpts} ===", flush=True)
    pats = []
    for c in ckpts:
        pats += [f"checkpoints/{c}/net.pt", f"checkpoints/{c}/arch.json"]
    snapshot_download(repo_id="nerijs/playmat-ai-checkpoints", repo_type="model", local_dir=work, allow_patterns=pats, token=tok)
    sds = []
    arch = None
    for c in ckpts:
        d = os.path.join(work, "checkpoints", c)
        if arch is None:
            arch = json.load(open(os.path.join(d, "arch.json")))
        sds.append(torch.load(os.path.join(d, "net.pt"), map_location="cpu", weights_only=True))
    avg = {}
    for k in sds[0].keys():
        if sds[0][k].is_floating_point():
            avg[k] = sum(sd[k].float() for sd in sds) / len(sds)
            avg[k] = avg[k].to(sds[0][k].dtype)
        else:
            avg[k] = sds[0][k]
    out_dir = os.path.join(work, "out", out_name)
    os.makedirs(out_dir, exist_ok=True)
    torch.save(avg, os.path.join(out_dir, "net.pt"))
    with open(os.path.join(out_dir, "arch.json"), "w") as f:
        json.dump(arch, f, indent=2)
    net = model.load_dir(out_dir, torch.device("cpu")).eval()
    model.export(net, os.path.join(out_dir, "net.pmnet"))
    with open(os.path.join(out_dir, "report.json"), "w") as f:
        json.dump({"soup_members": ckpts, "arch": arch}, f, indent=2)
    api = HfApi(token=tok)
    api.upload_folder(
        folder_path=out_dir,
        path_in_repo=f"checkpoints/{out_name}",
        repo_id="nerijs/playmat-ai-checkpoints",
        repo_type="model",
        commit_message=f"Model Soup: {out_name} ({len(ckpts)} members)",
    )
    print(f"UPLOADED {out_name} OK", flush=True)

make_soup("wavec-final-1.3m-soup", [
    "wavec-r11-1.3m-sem-oracle",
    "wavec-r12-1.3m-sem-oracle",
    "wavec-r14-1.3m-sem-oracle",
    "wavec-r15-1.3m-final",
])
make_soup("wavec-final-5m-soup", [
    "wavec-r11-5m-sem-oracle",
    "wavec-r12-5m-sem-oracle",
    "wavec-r12b-5m-widepol",
    "wavec-r13-5m-sem-oracle",
])
print("ALL_SOUPS_DONE", flush=True)


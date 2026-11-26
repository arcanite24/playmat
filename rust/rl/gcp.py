"""A Spot VM on Google Cloud as one more fleet host for a round (fleet.py drives it like the LAN's machines).

    python gcp.py up --project P [--type t2d-standard-16] [--zone us-central1-a] --bin selfplay-linux
    python gcp.py status --project P
    python gcp.py down --project P

`up` creates the VM (Spot: about a third of the on-demand price, and Google may take it back at any time), opens ssh
to it from this machine's public IP only, writes its host keys (read through gcloud, so nothing is trusted on first
use) and a `gcp-sp1` alias into ~/.ssh, and provisions it with fleet.py. Then a round takes it like any host:

    python rounds.py ... --lan minivac:8,blindvac:4,steamdeck:8,gcp-sp1:16

`down` deletes the VM, its firewall rule and the alias. The VM deletes itself after --hours (default 8) in any case,
so a forgotten one can't run up the bill. A preempted VM stops answering; its driver waits and the round goes on
without it. The x86 machine types (t2d, n2d, c2d, c3d) take the Linux selfplay built in rust:1.94-slim-bookworm.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.request

import fleet

NAME = "playmat-sp1"
ALIAS = "gcp-sp1"
USER = "playmat"
TAG = "playmat-selfplay"
RULE = "playmat-selfplay-ssh"
SSH_DIR = os.path.join(os.path.expanduser("~"), ".ssh")
CONFIG = os.path.join(SSH_DIR, "config")
KNOWN = os.path.join(SSH_DIR, "known_hosts_gcp")
BEGIN, END = "# playmat gcp begin (gcp.py writes this block)", "# playmat gcp end"


def gcloud(args, project, check=True, timeout=600):
    exe = shutil.which("gcloud") or sys.exit("gcloud isn't installed (https://cloud.google.com/sdk/docs/install)")
    p = subprocess.run([exe, *args, f"--project={project}", "--quiet"], capture_output=True, timeout=timeout)
    if check and p.returncode != 0:
        raise RuntimeError(f"gcloud {' '.join(args[:3])}: {p.stderr.decode(errors='replace').strip()[-600:]}")
    return p.stdout.decode(errors="replace").strip()


def public_ip():
    with urllib.request.urlopen("https://api.ipify.org", timeout=15) as r:
        ip = r.read().decode().strip()
    if not re.fullmatch(r"\d+\.\d+\.\d+\.\d+", ip):
        sys.exit(f"couldn't tell this machine's public IP ({ip!r}); pass --allow-from")
    return ip


def pubkey():
    with open(os.path.join(SSH_DIR, "id_ed25519.pub")) as f:
        return " ".join(f.read().split()[:2])


def write_alias(ip):
    """The alias block in ~/.ssh/config, replacing an earlier one: strict host-key checking against KNOWN."""
    try:
        with open(CONFIG) as f:
            text = f.read()
    except OSError:
        text = ""
    if text and not os.path.exists(CONFIG + ".bak-gcp"):
        shutil.copyfile(CONFIG, CONFIG + ".bak-gcp")
    text = re.sub(rf"\n?{re.escape(BEGIN)}.*?{re.escape(END)}\n?", "\n", text, flags=re.S).rstrip("\n")
    if ip:
        text += (f"\n\n{BEGIN}\nHost {ALIAS}\n  HostName {ip}\n  User {USER}\n  IdentityFile ~/.ssh/id_ed25519\n"
                 f"  IdentitiesOnly yes\n  StrictHostKeyChecking yes\n  UserKnownHostsFile ~/.ssh/known_hosts_gcp\n{END}")
    with open(CONFIG, "w", newline="\n") as f:
        f.write(text + "\n")


def host_keys(project, zone, ip, wait_s=240):
    """The VM's host keys as its guest agent publishes them (read over gcloud's authenticated API)."""
    end = time.time() + wait_s
    while True:
        out = gcloud(["compute", "instances", "get-guest-attributes", NAME, f"--zone={zone}",
                      "--query-path=hostkeys/", "--format=value(key,value)"], project, check=False)
        keys = [line.split("\t") for line in out.splitlines() if "\t" in line]
        if keys:
            return [f"{ip} {k} {v}" for k, v in keys]
        if time.time() > end:
            sys.exit("the VM published no host keys; not trusting it blind (python gcp.py down, then up again)")
        time.sleep(10)


def up(a):
    image = ["--image-family=debian-12-arm64" if a.type.startswith(("t2a", "c4a")) else "--image-family=debian-12",
             "--image-project=debian-cloud"]
    if a.type.startswith(("t2a", "c4a")) and not a.on_host:
        sys.exit("an ARM type needs an arm64 selfplay: not wired up yet, use an x86 type (t2d-standard-16)")
    allow = a.allow_from or public_ip()
    rules = gcloud(["compute", "firewall-rules", "list", f"--filter=name={RULE}", "--format=value(name)"], a.project)
    if rules:
        gcloud(["compute", "firewall-rules", "update", RULE, f"--source-ranges={allow}/32"], a.project)
    else:
        gcloud(["compute", "firewall-rules", "create", RULE, "--network=default", "--direction=INGRESS",
                "--allow=tcp:22", f"--source-ranges={allow}/32", f"--target-tags={TAG}"], a.project)
    gcloud(["compute", "instances", "create", NAME, f"--zone={a.zone}", f"--machine-type={a.type}", *image,
            "--provisioning-model=SPOT", "--instance-termination-action=DELETE", f"--max-run-duration={a.hours}h",
            f"--boot-disk-size={a.disk_gb}GB", "--boot-disk-type=pd-balanced", f"--tags={TAG}", "--labels=playmat=selfplay",
            "--no-service-account", "--no-scopes",
            f"--metadata=ssh-keys={USER}:{pubkey()},block-project-ssh-keys=TRUE,enable-guest-attributes=TRUE,enable-oslogin=FALSE"],
           a.project)
    ip = gcloud(["compute", "instances", "describe", NAME, f"--zone={a.zone}",
                 "--format=value(networkInterfaces[0].accessConfigs[0].natIP)"], a.project)
    with open(KNOWN, "w", newline="\n") as f:
        f.write("\n".join(host_keys(a.project, a.zone, ip)) + "\n")
    write_alias(ip)
    print(f"{NAME} up at {ip} ({a.type}, {a.zone}); ssh {ALIAS}", flush=True)
    # the guest agent adds the user a little after the host keys, and sshd restarts once during first boot: wait
    # for three logins in a row, 10 s apart
    ok = 0
    for _ in range(40):
        ok = ok + 1 if fleet.run([fleet.remote.SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", ALIAS, "true"],
                                 30, check=False).returncode == 0 else 0
        if ok == 3:
            break
        time.sleep(10)
    else:
        sys.exit(f"ssh {ALIAS} doesn't answer steadily")
    if a.bin or a.on_host:
        for attempt in range(3):
            try:
                print(fleet.provision(fleet.Host(ALIAS, 1, 0), a.bin, a.on_host), flush=True)
                break
            except RuntimeError as e:
                if attempt == 2:
                    raise
                print(f"provision failed ({e}); again in 20 s", flush=True)
                time.sleep(20)


def status(a):
    print(gcloud(["compute", "instances", "list", "--filter=labels.playmat=selfplay",
                  "--format=table(name,zone.basename(),machineType.basename(),status,scheduling.provisioningModel,"
                  "networkInterfaces[0].accessConfigs[0].natIP)"], a.project) or "no playmat VMs")


def down(a):
    gcloud(["compute", "instances", "delete", NAME, f"--zone={a.zone}"], a.project, check=False)
    gcloud(["compute", "firewall-rules", "delete", RULE], a.project, check=False)
    write_alias(None)
    print(f"{NAME} and {RULE} deleted; alias {ALIAS} removed")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=["up", "status", "down"])
    ap.add_argument("--project", required=True)
    ap.add_argument("--zone", default="us-central1-a")
    ap.add_argument("--type", default="t2d-standard-16")
    ap.add_argument("--hours", type=int, default=8, help="the VM deletes itself after this long")
    ap.add_argument("--disk-gb", type=int, default=20, help="boot disk size in GB")
    ap.add_argument("--allow-from", help="the IP ssh is opened to (default: this machine's public IP)")
    ap.add_argument("--bin", help="up: the selfplay to provision (Linux, for the VM's architecture)")
    ap.add_argument("--on-host", help=argparse.SUPPRESS)
    a = ap.parse_args()
    {"up": up, "status": status, "down": down}[a.cmd](a)


if __name__ == "__main__":
    main()

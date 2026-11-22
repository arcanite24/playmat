"""Self-play on the LAN's idle machines (minivac, blindvac...), driven by the trainer over ssh. Unlike the public
server (rl-worker.sh, which guards a production service on its own), these are machines the trainer already reaches
with ordinary keys, so nothing is installed on them but `selfplay`, the deck suite and a round's networks, all under
~/playmat-rl. While the round's local games run, one thread per host waits for the host to be idle, plays one short
chunk at the lowest priority, checks every 15 s that the host is still idle (a chunk is killed and thrown away the
moment it isn't), and copies the finished chunk back with scp.

    python fleet.py provision HOST --bin PATH      the trainer's selfplay (built for HOST) and the suite onto HOST
    python fleet.py provision HOST --on-host PATH  the same with a selfplay built on HOST itself (a Mac)
    python fleet.py status HOST [HOST...]          what each host is: its OS, whether it's idle and why not
    python fleet.py speed HOST --net DIR           one chunk with DIR's network, timed (provision first)

Idle: a Mac when nobody has touched its keyboard or mouse for IDLE_MAC_S seconds and its load is low; Windows when
the other processes use little CPU and it's on AC power; Linux (the Steam Deck) when it's on AC power, no Steam
game is running and its load is low. A host's selfplay must be the trainer's own build: provision
records the binary's sha256 and the trainer's commit, and a round checks both before it uses the host.
"""

import argparse
import base64
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import time

import remote

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
STATE = os.path.join(HERE, "data", "fleet.json")
IDLE_MAC_S = 600
LOAD_MAX = 0.35  # a Mac's 1-minute load per core before a chunk (its own games excluded while one runs)
CPU_OTHER_MAX = 40  # percent of a Windows host's CPU other processes may use
POLL_S = 15
CHUNK_GAMES = 100
SEED_BLOCK = 100_000  # each host's seeds in a round: 1_000_000 * r + 100_000 * (1 + index), so never the local
                      # games' (the first 100,000) nor the public server's (from 500,000)
GIT_SCP = r"C:\Program Files\Git\usr\bin\scp.exe"
SCP = os.environ.get("PLAYMAT_RL_SCP") or (GIT_SCP if remote.SSH.endswith("ssh.exe") and os.path.exists(GIT_SCP)
                                           else "scp")


def run(args, timeout=600, check=True):
    p = subprocess.run(args, capture_output=True, timeout=timeout)
    if check and p.returncode != 0:
        raise RuntimeError(f"{' '.join(args[:4])}...: {p.stderr.decode(errors='replace').strip()[-400:]}")
    return p


class Host:
    """One LAN machine: `alias` (an ssh alias), `jobs` (selfplay's threads), `index` (its seed block)."""

    def __init__(self, alias, jobs, index):
        self.alias, self.jobs, self.index = alias, jobs, index
        self.os = None
        self.home = None

    def sh(self, cmd, timeout=120, check=True):
        """A command on the host: sh on a Mac or Linux, PowerShell (sent encoded) on Windows."""
        if self.windows():
            enc = base64.b64encode(cmd.encode("utf-16-le")).decode()
            cmd = f"powershell -NoProfile -NonInteractive -EncodedCommand {enc}"
        p = run([remote.SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", self.alias, cmd], timeout, check)
        return p.stdout.decode(errors="replace").replace("\r", "")

    def detect(self):
        if self.os is None:
            p = run([remote.SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", self.alias, "uname -s"], 60,
                    check=False)
            out = p.stdout.decode(errors="replace").strip()
            self.os = {"Darwin": "mac", "Linux": "linux"}.get(out, "windows")
        return self.os

    def windows(self):
        return self.detect() == "windows"

    def path(self, *parts):
        """A path under the host's ~/playmat-rl, spelled out (no variables: it goes inside quoted arguments)."""
        if self.home is None:
            self.home = self.sh("$env:USERPROFILE" if self.windows() else "echo $HOME", timeout=30).strip()
        sep = "\\" if self.windows() else "/"
        return self.home + sep + "playmat-rl" + "".join(sep + p for p in parts)

    def scp_to(self, local, rel):
        run([SCP, "-q", "-o", "BatchMode=yes", local, f"{self.alias}:playmat-rl/{rel}"], timeout=900)

    def scp_from(self, rel, local):
        run([SCP, "-q", "-r", "-o", "BatchMode=yes", f"{self.alias}:playmat-rl/{rel}", local], timeout=900)

    def exe(self):
        return self.path("selfplay.exe" if self.windows() else "selfplay")

    def busy(self, playing=False):
        """Why the host isn't idle, or None. While a chunk plays, its own games don't count."""
        if self.detect() == "mac":
            out = self.sh("ioreg -c IOHIDSystem | awk '/HIDIdleTime/ {print int($NF/1000000000); exit}'; "
                          "sysctl -n vm.loadavg hw.ncpu", timeout=30)
            vals = out.split()
            idle_s, load1, ncpu = int(vals[0]), float(vals[2]), int(vals[-1])
            if idle_s < (60 if playing else IDLE_MAC_S):
                return f"in use ({idle_s} s since the last input)"
            if not playing and load1 / ncpu > LOAD_MAX:
                return f"load {load1:.1f} on {ncpu} cores"
            return None
        if self.windows():
            out = self.sh(
                "$b = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue | Select-Object -First 1; "
                "$ac = if ($b) { $b.BatteryStatus -eq 2 } else { $true }; "
                "$n = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors; "
                "$s = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | "
                "Where-Object { $_.Name -notmatch '^(_Total|Idle|selfplay.*)$' } | Measure-Object PercentProcessorTime -Sum; "
                "'{0} {1:N0}' -f $ac, ($s.Sum / $n)", timeout=60)
            ac, other = out.split()[:2]
            if ac != "True":
                return "on battery"
            if float(other.replace(",", "")) > CPU_OTHER_MAX:
                return f"other processes use {other}% CPU"
            return None
        # Linux (the Steam Deck): on mains power, no Steam game running (Steam starts every game under
        # `reaper SteamLaunch`; the brackets keep pgrep from matching this command), and a low load before a chunk
        out = self.sh("cat /sys/class/power_supply/AC*/online 2>/dev/null | head -1; "
                      "pgrep -f '[r]eaper SteamLaunch' > /dev/null && echo game || echo nogame; "
                      "cat /proc/loadavg; nproc", timeout=30).split()
        if out and out[0] == "0":
            return "on battery"
        if "game" in out:
            return "a Steam game is running"
        out = out[out.index("nogame") + 1:] if "nogame" in out else out
        if not playing and float(out[0]) / int(out[-1]) > LOAD_MAX:
            return f"load {out[0]} on {out[-1]} cores"
        return None

    def kill(self):
        if self.windows():
            self.sh(f"Get-Process selfplay -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -eq '{self.exe()}' }} "
                    "| Stop-Process -Force", check=False)
        else:
            self.sh(f"pkill -f '{self.exe()}' || true", check=False)


def commit():
    try:
        return run(["git", "-C", REPO, "rev-parse", "HEAD"], 30).stdout.decode().strip()
    except Exception:
        return None


def load_state():
    try:
        with open(STATE) as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


def provision(host, binary=None, on_host=None):
    """The selfplay binary (built for the host: `binary` here, or `on_host`, a path on the host) and the deck suite
    onto the host, recorded with the trainer's commit."""
    h = host
    if h.windows():
        h.sh(f"New-Item -ItemType Directory -Force '{h.path('repo', 'archbench', 'constructed-2')}', "
             f"'{h.path('repo', 'Mage.Server', 'career-src')}', '{h.path('job')}', '{h.path('out')}' | Out-Null")
    else:
        h.sh(f"mkdir -p '{h.path('repo', 'archbench', 'constructed-2')}' '{h.path('repo', 'Mage.Server', 'career-src')}' "
             f"'{h.path('job')}' '{h.path('out')}'")
    if on_host:
        if h.windows():
            h.sh(f"Copy-Item -Force '{on_host}' '{h.exe()}'")
        else:
            h.sh(f"cp '{on_host}' '{h.exe()}'")
    else:
        h.scp_to(binary, "selfplay.exe" if h.windows() else "selfplay")
    if not h.windows():
        h.sh(f"chmod 755 '{h.exe()}'")
    elif os.name == "nt":
        # the Visual C++ runtime, next to the exe (app-local), for hosts that don't have it installed
        for dll in ("vcruntime140.dll", "vcruntime140_1.dll"):
            src = os.path.join(os.environ.get("SystemRoot", r"C:\Windows"), "System32", dll)
            if os.path.exists(src):
                h.scp_to(src, dll)
    h.scp_to(os.path.join(REPO, "archbench", "constructed-2", "suite.json"), "repo/archbench/constructed-2/suite.json")
    for deck_dir in ("pioneer", "standard"):
        run([SCP, "-q", "-r", "-o", "BatchMode=yes", os.path.join(REPO, "Mage.Server", "career-src", deck_dir),
             f"{h.alias}:playmat-rl/repo/Mage.Server/career-src/"], timeout=900)
    state = load_state()
    state[h.alias] = {"sha256": host_sha256(h) if on_host else remote.sha256(binary), "commit": commit(), "os": h.os, "at": time.strftime("%FT%TZ", time.gmtime())}
    os.makedirs(os.path.dirname(STATE), exist_ok=True)
    with open(STATE, "w") as f:
        json.dump(state, f, indent=1)
    return state[h.alias]


def check(host):
    """None when the host has the trainer's own selfplay (the binary for its platform, from this commit); else why not."""
    rec = load_state().get(host.alias)
    if rec is None:
        return "not provisioned (python fleet.py provision)"
    if rec.get("commit") != commit():
        return f"provisioned from {str(rec.get('commit'))[:9]}, the trainer is at {str(commit())[:9]}"
    if host_sha256(host) != rec.get("sha256"):
        return "its selfplay changed since provision"
    return None


def host_sha256(host):
    exe = host.exe()
    out = host.sh(f"(Get-FileHash '{exe}' -Algorithm SHA256).Hash" if host.windows()
                  else f"shasum -a 256 '{exe}' 2>/dev/null || sha256sum '{exe}'", timeout=120).split()
    return out[0].lower() if out else None


class Driver(threading.Thread):
    """Plays chunks of one round on one host while it's idle, until stop(); finished chunks land in `dest` as
    <alias>-c<n>-<file>."""

    def __init__(self, host, round_no, learner, net, rollout, keep, fast, tag, dest, log=print):
        super().__init__(daemon=True)
        self.h, self.r, self.learner, self.keep, self.fast, self.tag, self.dest = host, round_no, learner, keep, fast, tag, dest
        self.net, self.rollout, self.log = net, rollout, log
        self.stopping = threading.Event()
        self.games = self.chunks = self.stopped = 0
        self.problem = None

    def stop(self):
        self.stopping.set()

    def remote_learner(self):
        net, roll = self.h.path("job", "net.pmnet"), self.h.path("job", "rollout.pmnet")
        return self.learner.replace(self.rollout, roll).replace(self.net, net)

    def chunk_cmd(self, seed, n):
        args = ["--suite", self.h.path("repo", "archbench", "constructed-2"), "--repo", self.h.path("repo"),
                "--out", self.h.path("out", f"c{n:06d}"), "--games", str(CHUNK_GAMES), "--jobs", str(self.h.jobs),
                "--keep", str(self.keep), "--learner", self.remote_learner(), "--opponents", "self",
                "--seed", str(seed), "--tag", str(self.tag)] + (["--fast", self.fast] if self.fast else [])
        out = self.h.path("out", f"c{n:06d}")
        if self.h.windows():
            quoted = " ".join(f'"{a}"' for a in args)
            return (f"Remove-Item -Recurse -Force \"{out}\" -ErrorAction SilentlyContinue; "
                    f"New-Item -ItemType Directory -Force \"{out}\" | Out-Null; "
                    f"$p = Start-Process -FilePath \"{self.h.exe()}\" -ArgumentList '{quoted.replace(chr(39), chr(39) * 2)}' "
                    f"-NoNewWindow -PassThru -RedirectStandardOutput \"{out}.log\" -RedirectStandardError \"{out}.err\"; "
                    "$p.PriorityClass = 'Idle'; $p.WaitForExit(); "
                    f"Get-Content \"{out}.log\" | Select-String '^SELFPLAY [0-9]+ games' | Select-Object -First 1 | % Line; "
                    "exit $p.ExitCode")
        quoted = " ".join(f'"{a}"' for a in args)
        # the trainer keeps the channel's stdin open while it waits; when it goes (killed, link lost), stdin ends
        # and the chunk is killed with it
        return (f"rm -rf '{out}' && mkdir -p '{out}' && exec 3<&0 && "
                f"{{ nice -n 20 '{self.h.exe()}' {quoted} > '{out}.log' 2>&1 & }} && pid=$! && "
                f"{{ (cat <&3 > /dev/null; kill $pid 2> /dev/null) & }} && w=$! && "
                f"wait $pid; c=$?; kill $w 2> /dev/null; grep -m1 '^SELFPLAY [0-9]* games' '{out}.log'; exit $c")

    def run(self):
        h = self.h
        try:
            for f, name in ((self.net, "net.pmnet"), (self.rollout, "rollout.pmnet")):
                h.scp_to(f, f"job/{name}")
        except Exception as e:
            self.problem = f"sending the networks: {e}"
            self.log(f"FLEET {h.alias}: {self.problem}")
            return
        n = 0
        last_end = 0.0  # when a chunk last finished on its own
        while not self.stopping.is_set():
            try:
                # straight after a chunk the load is still its own games, so the next one is checked as if it were
                # still playing (a Mac: no input for a minute; Linux: no load check); otherwise back-to-back chunks
                # each wait for the load average to decay, and a host plays a quarter of the time
                why = h.busy(playing=time.time() - last_end < 120)
            except Exception as e:
                why = f"no answer ({e})"
            if why:
                self.stopping.wait(60)
                continue
            seed = 1_000_000 * self.r + SEED_BLOCK * (1 + h.index) + n * CHUNK_GAMES
            if n * CHUNK_GAMES >= SEED_BLOCK:
                self.log(f"FLEET {h.alias}: its seed block is used up")
                return
            result = {}
            t = threading.Thread(target=self._play, args=(seed, n, result), daemon=True)
            t0 = time.time()
            t.start()
            killed = None
            while t.is_alive():
                t.join(POLL_S)
                if not t.is_alive():
                    break
                try:
                    why = h.busy(playing=True)
                except Exception as e:
                    why = f"no answer ({e})"
                if why:
                    killed = why
                    h.kill()
                    t.join(60)
                    break
            if killed or result.get("code") != 0 or not result.get("summary"):
                self.stopped += 1
                self.log(f"FLEET {h.alias}: chunk c{n:06d} thrown away: {killed or result.get('error') or 'selfplay exited ' + str(result.get('code'))}")
                n += 1
                continue
            t1 = last_end = time.time()
            try:
                self._fetch(n)
                self.games += CHUNK_GAMES
                self.chunks += 1
                self.log(f"FLEET {h.alias}: chunk c{n:06d}, {CHUNK_GAMES} games in {time.time() - t0:.0f} s "
                         f"(played {t1 - t0:.0f} s, fetched {time.time() - t1:.0f} s; {result['summary'][9:70]})")
            except Exception as e:
                self.stopped += 1
                self.log(f"FLEET {h.alias}: chunk c{n:06d} not fetched: {e}")
            n += 1

    def _play(self, seed, n, result):
        try:
            if self.h.windows():
                out = self.h.sh(self.chunk_cmd(seed, n), timeout=1800, check=False)
            else:  # stdin stays open (and the chunk alive) only as long as this ssh does
                p = subprocess.Popen([remote.SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", self.h.alias,
                                      self.chunk_cmd(seed, n)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL)
                out = p.stdout.read().decode(errors="replace")  # until the chunk ends; stdin is never written
                p.stdin.close()
                p.wait(60)
            result["summary"] = next((line for line in out.splitlines() if line.startswith("SELFPLAY ")), None)
            result["code"] = 0 if result["summary"] else 1
        except Exception as e:
            result["error"] = str(e)

    def _fetch(self, n):
        tmp = tempfile.mkdtemp(dir=self.dest)
        try:
            if self.h.windows():
                self.h.scp_from(f"out/c{n:06d}", tmp)
            else:
                # one compressed stream, not a round trip per file: over the internet (a cloud VM) scp spends most
                # of a chunk's time on its ~200 files
                p = run([remote.SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", self.h.alias,
                         f"COPYFILE_DISABLE=1 tar czf - -C '{self.h.path('out')}' c{n:06d}"], timeout=900)
                os.makedirs(os.path.join(tmp, f"c{n:06d}"))
                with tarfile.open(fileobj=io.BytesIO(p.stdout), mode="r:gz") as tar:
                    for m in tar.getmembers():
                        if (m.isfile() and os.path.dirname(m.name) == f"c{n:06d}"
                                and not os.path.basename(m.name).startswith("._")):  # a Mac's AppleDouble files
                            with open(os.path.join(tmp, m.name), "wb") as f:
                                shutil.copyfileobj(tar.extractfile(m), f)
            src = os.path.join(tmp, f"c{n:06d}")
            for name in os.listdir(src):
                if os.path.isfile(os.path.join(src, name)):
                    os.replace(os.path.join(src, name), os.path.join(self.dest, f"{self.h.alias}-c{n:06d}-{name}"))
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        if self.h.windows():
            self.h.sh(f"Remove-Item -Recurse -Force \"{self.h.path('out', f'c{n:06d}')}\"*", check=False)
        else:
            self.h.sh(f"rm -rf '{self.h.path('out', f'c{n:06d}')}' '{self.h.path('out', f'c{n:06d}')}.log'", check=False)


def parse_hosts(spec):
    """`minivac:8,blindvac:4` -> Hosts (jobs default 4)."""
    hosts = []
    for i, part in enumerate(p for p in (spec or "").split(",") if p.strip()):
        alias, _, jobs = part.strip().partition(":")
        hosts.append(Host(alias, int(jobs or 4), i))
    if len(hosts) > 4:
        sys.exit("at most 4 LAN hosts (their seed blocks sit between the local games and the public server's)")
    return hosts


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=["provision", "status", "speed"])
    ap.add_argument("hosts", nargs="+", help="ssh aliases (alias:jobs)")
    ap.add_argument("--bin", help="provision: the selfplay binary built for that host")
    ap.add_argument("--on-host", help="provision: a selfplay already built on the host (its path there)")
    ap.add_argument("--net", help="speed: a run folder with net.pmnet and rollout/net.pmnet")
    a = ap.parse_args()
    hosts = parse_hosts(",".join(a.hosts))
    if a.cmd == "provision":
        if not (a.bin or a.on_host) or len(hosts) != 1:
            sys.exit("provision HOST --bin PATH (or --on-host PATH)")
        print(json.dumps(provision(hosts[0], a.bin, a.on_host)))
    elif a.cmd == "status":
        for h in hosts:
            try:
                print(f"{h.alias}: {h.detect()}, {h.busy() or 'idle'}")
            except Exception as e:
                print(f"{h.alias}: {e}")
    else:
        if not a.net:
            sys.exit("speed HOST --net DIR")
        net, roll = os.path.join(a.net, "net.pmnet"), os.path.join(a.net, "rollout", "net.pmnet")
        learner = f"cfg:policy=netsearch+hide=1+samples=4+eval=2+net={net}+rollout={roll}+temp=1000"
        dest = tempfile.mkdtemp()
        d = Driver(hosts[0], 0, learner, net, roll, 500, "net", 0, dest)
        d.start()
        while d.is_alive() and d.chunks + d.stopped == 0:
            time.sleep(5)
        d.stop()
        d.join(5)
        print(f"{hosts[0].alias}: {d.chunks} chunks, {d.games} games, {len(os.listdir(dest))} files")
        shutil.rmtree(dest, ignore_errors=True)


if __name__ == "__main__":
    main()

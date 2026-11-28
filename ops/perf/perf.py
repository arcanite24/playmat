#!/usr/bin/env python3
"""Helpers for perf.sh (standard library only).

  perf.py config <config.xml> <port> <websocketPort>   point a distribution's config at the harness ports and lift its game limits
  perf.py jfc <java home> <out.jfc>                    JFR "profile" settings minus the costly GC-phase and native-sample events
  perf.py report <jfr tool> <recording.jfr>            CPU and allocation breakdown; also writes cpu-categories.json next to it
  perf.py summarize <results dir>                      fold facts, samples, load output and the JFR breakdown into summary.json
  perf.py compare <results dir> <results dir>          side-by-side numbers of two runs
"""
import collections
import json
import os
import re
import subprocess
import sys

# First match wins, so order matters: replays and AI search call into view building and playables themselves,
# and deck loading and card-database queries run under bridge RPC frames.
CATEGORIES = [
    ("bridge state diffs", ("mage.server.websocket.GameStates.encode",)),
    ("replay recording", ("mage.server.replay.",)),
    ("AI search", ("mage.player.ai.",)),
    ("playable highlighting", ("getPlayableObjects", "createSimulationForPlayableCalc")),
    ("deck loading", ("mage.cards.decks.Deck.load",)),
    ("card database", ("org.h2.", "com.j256.ormlite.")),
    ("game views", ("mage.view.",)),
    ("message serialization", ("mage.server.websocket.", "com.google.gson.", "org.java_websocket.")),
    ("desktop transport", ("org.jboss.",)),
    ("rules engine", ("mage.",)),
]
DISABLED_EVENTS = ("jdk.NativeMethodSample", "jdk.GCPhaseParallel", "jdk.PromoteObjectInNewPLAB",
                   "jdk.PromoteObjectOutsidePLAB", "jdk.TenuringDistribution")


def config(path, port, ws_port):
    xml = open(path, encoding="utf-8").read()
    values = {"port": port, "websocketPort": ws_port, "secondaryBindPort": "-1", "maxGameThreads": "64",
              "maxAiOpponents": "100", "maxSecondsIdle": "600"}
    start = xml.index("<server ")
    end = xml.index(">", start)
    tag = xml[start:end]
    closing = "/" if tag.rstrip().endswith("/") else ""
    tag = tag.rstrip().rstrip("/").rstrip() if closing else tag
    for name, value in values.items():
        if re.search(rf'\s{name}="[^"]*"', tag):
            tag = re.sub(rf'(\s){name}="[^"]*"', rf'\g<1>{name}="{value}"', tag)
        else:
            tag += f' {name}="{value}"'
    open(path, "w", encoding="utf-8").write(xml[:start] + tag + closing + xml[end:])


def jfc(java_home, out):
    text = open(os.path.join(java_home, "lib", "jfr", "profile.jfc"), encoding="utf-8").read()
    for event in DISABLED_EVENTS:
        text = re.sub(rf'(<event name="{re.escape(event)}">.*?<setting name="enabled"[^>]*>)true(</setting>)',
                      r"\1false\2", text, flags=re.S)
    open(out, "w", encoding="utf-8").write(text)


def events(jfr, recording, kind):
    out = subprocess.run([jfr, "print", "--json", "--stack-depth", "128", "--events", kind, recording],
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out)["recording"]["events"]


def frames(event):
    stack = event["values"].get("stackTrace") or {}
    return [f["method"]["type"]["name"].replace("/", ".") + "." + f["method"]["name"] for f in stack.get("frames", [])]


def thread_name(event):
    t = event["values"].get("sampledThread") or event["values"].get("eventThread") or {}
    return re.sub(r"\d+", "#", t.get("javaName") or t.get("osName") or "?")


def category(stack):
    joined = " ".join(stack)
    for name, markers in CATEGORIES:
        if any(m in joined for m in markers):
            return name
    return "JVM and other"


def share(counter, total, n):
    return [f"{v * 100 / max(total, 1):5.1f}%  {k}" for k, v in counter.most_common(n)]


def report(jfr, recording, top=25):
    lines = []
    samples = events(jfr, recording, "jdk.ExecutionSample")
    total = len(samples)
    cats, threads, leaf, incl = (collections.Counter() for _ in range(4))
    for e in samples:
        fs = frames(e)
        if not fs:
            continue
        cats[category(fs)] += 1
        threads[thread_name(e)] += 1
        leaf[fs[0]] += 1
        for m in set(fs):
            if m.startswith("mage."):
                incl[m] += 1
    lines += [f"## CPU: {total} execution samples", "", "### By category (first match wins)"] + share(cats, total, 20)
    lines += ["", "### By thread"] + share(threads, total, 12)
    lines += ["", f"### Top {top} leaf methods"] + share(leaf, total, top)
    lines += ["", f"### Top {top * 2} inclusive mage.* methods"] + share(incl, total, top * 2)

    allocs = events(jfr, recording, "jdk.ObjectAllocationSample")
    weight = sum(e["values"].get("weight", 0) for e in allocs)
    acats, classes, sites = (collections.Counter() for _ in range(3))
    for e in allocs:
        w = e["values"].get("weight", 0)
        fs = frames(e)
        acats[category(fs)] += w
        classes[e["values"].get("objectClass", {}).get("name", "?").replace("/", ".")] += w
        ours = [f for f in fs if f.startswith(("mage.", "com.google.gson", "org.h2"))]
        sites[" <- ".join((ours or fs)[:4])] += w
    lines += ["", f"## Allocation: {len(allocs)} samples, {weight / 1e9:.2f} GB sampled weight", "", "### By category"]
    lines += share(acats, weight, 20)
    lines += ["", f"### Top {top} classes"] + share(classes, weight, top)
    lines += ["", f"### Top {top} sites"] + share(sites, weight, top)
    print("\n".join(lines))

    out = os.path.join(os.path.dirname(recording), "cpu-categories.json")
    json.dump({"cpu": {k: round(v * 100 / max(total, 1), 1) for k, v in cats.most_common()},
               "alloc": {k: round(v * 100 / max(weight, 1), 1) for k, v in acats.most_common()},
               "allocGB": round(weight / 1e9, 2)}, open(out, "w"), indent=2)


def read_facts(run):
    facts = collections.defaultdict(list)
    for line in open(os.path.join(run, "facts.txt"), encoding="utf-8"):
        if "=" in line:
            key, value = line.rstrip("\n").split("=", 1)
            facts[key].append(value)
    return facts


def number(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def summarize(run):
    facts = read_facts(run)
    s = {}
    for key, values in facts.items():
        nums = [number(v) for v in values]
        if all(n is not None for n in nums):
            s[key] = nums[0] if len(nums) == 1 else nums
        else:
            s[key] = values[0] if len(values) == 1 else values
    samples = os.path.join(run, "samples.txt")
    if os.path.exists(samples):
        rows = [r.split() for r in open(samples).read().splitlines()[1:] if len(r.split()) == 4]
        if len(rows) > 1:
            t = [float(r[0]) for r in rows]
            cpu = [float(r[1]) for r in rows]
            s["peakRssMB"] = max(float(r[2]) for r in rows)
            s["peakThreads"] = max(int(r[3]) for r in rows)
            s["meanCores"] = round((cpu[-1] - cpu[0]) / max(t[-1] - t[0], 1), 2)
    log = os.path.join(run, "load.log")
    if os.path.exists(log):
        text = open(log, encoding="utf-8", errors="replace").read()
        m = re.search(r"^\[perf-result\] (\{.*\})$", text, re.M)
        if m:
            s["load"] = json.loads(m.group(1))
        m = re.search(r"finished (\d+)/(\d+)", text)
        if m and "load" not in s:
            s["load"] = {"scenario": "human", "games": int(m.group(1)), "of": int(m.group(2))}
    games = (s.get("load") or {}).get("games") if s.get("scenario") != "hold" else None
    if games and isinstance(s.get("loadCpuSec"), float):
        s["cpuSecPerGame"] = round(s["loadCpuSec"] / games, 2)
        if isinstance(s.get("gatewayLoadCpuSec"), float):
            s["gatewayCpuSecPerGame"] = round(s["gatewayLoadCpuSec"] / games, 3)
    if s.get("scenario") == "hold" and isinstance(s.get("heldLiveHeapMB"), float) and isinstance(s.get("beforeLiveHeapMB"), float):
        held = (s.get("load") or {}).get("finished") or (s.get("load") or {}).get("games")
        if held:
            s["liveHeapMBPerHeldGame"] = round((s["heldLiveHeapMB"] - s["beforeLiveHeapMB"]) / held, 2)
    cats = os.path.join(run, "cpu-categories.json")
    if os.path.exists(cats):
        s["profile"] = json.load(open(cats))
    json.dump(s, open(os.path.join(run, "summary.json"), "w"), indent=2)
    print(json.dumps(s, indent=2))


def flatten(d, prefix=""):
    out = {}
    for k, v in d.items():
        if isinstance(v, dict):
            out.update(flatten(v, f"{prefix}{k}."))
        elif isinstance(v, list) and v and all(isinstance(x, (int, float)) for x in v):
            out[f"{prefix}{k}"] = sum(v) / len(v)
        elif isinstance(v, (int, float)) and not isinstance(v, bool):
            out[f"{prefix}{k}"] = v
    return out


def compare(a, b):
    sa = flatten(json.load(open(os.path.join(a, "summary.json"))))
    sb = flatten(json.load(open(os.path.join(b, "summary.json"))))
    print(f"{'metric':<44} {os.path.basename(a.rstrip('/')):>22} {os.path.basename(b.rstrip('/')):>22} {'change':>8}")
    for key in sorted(set(sa) | set(sb)):
        va, vb = sa.get(key), sb.get(key)
        change = f"{(vb - va) * 100 / va:+.0f}%" if va and vb is not None else ""
        fmt = lambda v: "" if v is None else f"{v:.2f}"
        print(f"{key:<44} {fmt(va):>22} {fmt(vb):>22} {change:>8}")


if __name__ == "__main__":
    command, args = (sys.argv[1], sys.argv[2:]) if len(sys.argv) > 1 else ("help", [])
    commands = {"config": config, "jfc": jfc, "report": report, "summarize": summarize, "compare": compare}
    if command not in commands:
        print(__doc__)
        sys.exit(1)
    commands[command](*args)

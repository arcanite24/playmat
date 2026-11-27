"""External engine harness wrappers for Forge CLI (`sim`), XMage Java MAD (`RustAiGameTest`), and MageZero (`mz`).

These engines maintain their own rules implementations and cannot run directly inside Playmat's Rust `Table`, so
this module provides:
  1. Deck format converters (Playmat Archbench `.txt` -> Forge `.dck`).
  2. Availability probes (`probe_engines()`).
  3. Reproducible CLI runners that parse Forge / XMage / MageZero output into unified Archbench-style JSON records.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
from typing import Any, Dict, List, Optional

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))


def convert_txt_to_forge_dck(txt_path: str, out_dck_path: str, deck_name: Optional[str] = None) -> str:
    """Converts a Playmat/Archbench `.txt` deck file into Forge `.dck` format."""
    name = deck_name or os.path.splitext(os.path.basename(txt_path))[0]
    main_lines: List[str] = []
    sb_lines: List[str] = []
    with open(txt_path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#") or line.startswith("NAME:"):
                continue
            is_sb = line.startswith("SB:")
            if is_sb:
                line = line[3:].strip()
            m = re.match(r"^(\d+)\s+(?:\[[^\]]*\]\s+)?(.+?)(?:\s+\([^)]+\).*)?$", line)
            if not m:
                continue
            entry = f"{m.group(1)} {m.group(2).strip()}"
            (sb_lines if is_sb else main_lines).append(entry)

    os.makedirs(os.path.dirname(os.path.abspath(out_dck_path)), exist_ok=True)
    with open(out_dck_path, "w", encoding="utf-8", newline="\n") as out:
        out.write("[metadata]\n")
        out.write(f"Name={name}\n")
        out.write("[Main]\n")
        for line in main_lines:
            out.write(line + "\n")
        if sb_lines:
            out.write("[Sideboard]\n")
            for line in sb_lines:
                out.write(line + "\n")
    return out_dck_path


def probe_engines() -> Dict[str, Dict[str, Any]]:
    """Checks which external MTG engines / CLIs are available on this machine."""
    forge_jar = os.environ.get("FORGE_JAR") or ""
    forge_bin = shutil.which("forge") or ""
    mvn_bin = shutil.which("mvn") or ""
    java_bin = shutil.which("java") or ""
    mz_bin = shutil.which("mz") or ""
    ollama_bin = shutil.which("ollama") or ""
    return {
        "playmat_rust": {
            "available": os.path.exists(os.path.join(REPO, "rust", "target", "release", "archbench"))
            or os.path.exists(os.path.join(REPO, "rust", "target", "debug", "archbench")),
            "how": "cargo build --release -p playmat-ai --bins",
        },
        "xmage_java_mad": {
            "available": bool(mvn_bin and java_bin and os.path.exists(os.path.join(REPO, "Mage.Tests"))),
            "binary": mvn_bin or None,
            "how": "mvn -o -pl Mage.Tests test -Dtest=RustAiGameTest -Dxmage.rustAiGames.java=true",
        },
        "forge_cli": {
            "available": bool((forge_jar and os.path.exists(forge_jar) and java_bin) or forge_bin),
            "binary": forge_jar or forge_bin or None,
            "how": "Set FORGE_JAR=/path/to/forge-gui-desktop-jar-with-dependencies.jar",
        },
        "magezero": {
            "available": bool(mz_bin),
            "binary": mz_bin or None,
            "how": "Install MageZero v0.3.0-alpha (github.com/WillWroble/MageZero) so `mz` is on PATH",
        },
        "ollama_local_llms": {
            "available": bool(ollama_bin),
            "binary": ollama_bin or None,
            "how": "Use --entrant ollama=llm:ollama/<model> (local $0 open-weight LLMs)",
        },
    }


def run_forge_sim(deck_a_dck: str, deck_b_dck: str, games: int = 10, forge_jar: Optional[str] = None) -> Dict[str, Any]:
    """Runs Forge's headless `sim` mode between two `.dck` files and parses the match summary."""
    jar = forge_jar or os.environ.get("FORGE_JAR")
    if not jar or not os.path.exists(jar):
        raise RuntimeError("Forge JAR not found. Set FORGE_JAR=/path/to/forge-gui-desktop-jar-with-dependencies.jar")
    cmd = ["java", "-Xmx2048m", "-jar", jar, "sim", "-d", deck_a_dck, deck_b_dck, "-n", str(games)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    text = proc.stdout + "\n" + proc.stderr
    # Forge sim prints lines like: "Match Result: Ai(1)-DeckA: 6 Ai(2)-DeckB: 4"
    m = re.search(r"Ai\(1\)[^:]*:\s*(\d+)\s+Ai\(2\)[^:]*:\s*(\d+)", text)
    w_a, w_b = (int(m.group(1)), int(m.group(2))) if m else (0, 0)
    return {
        "harness": "forge-sim",
        "deck_a": os.path.basename(deck_a_dck),
        "deck_b": os.path.basename(deck_b_dck),
        "games": games,
        "a_won": w_a,
        "b_won": w_b,
        "returncode": proc.returncode,
    }


def run_xmage_java_mad(games: int = 48, threads: int = 4) -> Dict[str, Any]:
    """Runs XMage's Java MAD AI through `RustAiGameTest -Dxmage.rustAiGames.java=true`."""
    cmd = [
        "mvn",
        "-o",
        "-pl",
        "Mage.Tests",
        "test",
        "-Dtest=RustAiGameTest",
        "-Dsurefire.failIfNoSpecifiedTests=false",
        "-Dxmage.rustAiGames.java=true",
        f"-Dxmage.rustAiGames.games={games}",
        f"-Dxmage.rustAiGames.threads={threads}",
        "-Dxmage.dataCollectors.printGameLogs=false",
    ]
    proc = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    text = proc.stdout + "\n" + proc.stderr
    m = re.search(r"won\s+(\d+).*?drew\s+(\d+).*?lost\s+(\d+)", text, flags=re.IGNORECASE)
    w, d, l = (int(m.group(1)), int(m.group(2)), int(m.group(3))) if m else (0, 0, 0)
    return {
        "harness": "java-mage-tests",
        "entrant": "java-mad@s4",
        "games": games,
        "won": w,
        "drew": d,
        "lost": l,
        "returncode": proc.returncode,
    }

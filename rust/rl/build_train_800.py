#!/usr/bin/env python3
"""Generate and verify an 800-deck Phase 7 training suite (`archbench/constructed-train-800`)
with strict zero non-basic card leakage against `archbench/constructed-ood-1`.

Combines the 42 curated `constructed-2` Pioneer/Standard decks with 758 quality-gated,
archetype-structured 60-card decks built strictly from the 16,483 verified cards in
`card_registry_32k.txt` (excluding all 447 non-basic cards in `constructed-ood-1`):
  - 418 Constructed decks (60 mono-color, 200 guild, 120 tri-color, 38 tribal synergy)
  - 180 Limited-curve low-copy decks (120 guild, 60 tri-color; 1-2 copies, cohesive curve)
  - 160 Brawl/Singleton 60-card decks (20 mono, 80 guild, 60 tri-color; 1-of non-basics)

Fixes all Phase 6 `build_train_450.py` deck-construction flaws:
  1. `"mulligans": true` enabled in `archbench/constructed-train-800/suite.json`.
  2. Strict color identity + off-color activation/ability cost filtering across casting costs,
     split/adventure halves, back faces, `originalColorIdentity`, and rules text `{W/U/B/R/G}` pips.
  3. Karsten pip-weighted manabases: exact colored pip counting (weighting double/triple pips),
     dynamic MV-matched land counts (20-22 aggro/tempo, 23-24 midrange/limited, 25-26 control/bigmana,
     24-25 Brawl 60-card), and strict enters-tapped dual caps (<= 4 aggro/tempo, <= 8 midrange/control).
  4. Principled `card_score` + dead/parasitic card gating: eliminates MV<=3 draft-chaff trinket
     inflation, uncastable 0-cost/meld/colorless-pip bricks, narrow color hosers, wishboard/Commander
     cards, and 0-synergy orphan tribal lords in non-tribal decks.
  5. Automated fast quality gate: per-deck 200-hand Monte Carlo London-mulligan castability & curve
     coherence gate (< 15% unplayable hands), `decktool --check-career` (100% playable, 0 missing),
     and live `archbench` (`host@s6` + `rust-quick`) smoke verification with 0 engine failures.
"""

import argparse
import gzip
import json
import os
import random
import re
import subprocess
import tempfile
import time
from collections import Counter, defaultdict
from pathlib import Path

BASICS = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}
ALL_BASIC_NAMES = {
    "Plains",
    "Island",
    "Swamp",
    "Mountain",
    "Forest",
    "Wastes",
    "Snow-Covered Plains",
    "Snow-Covered Island",
    "Snow-Covered Swamp",
    "Snow-Covered Mountain",
    "Snow-Covered Forest",
}
COLOR_SET = {"W", "U", "B", "R", "G"}

MONO = [["W"], ["U"], ["B"], ["R"], ["G"]]
GUILDS = [
    ["W", "U"],
    ["U", "B"],
    ["B", "R"],
    ["R", "G"],
    ["G", "W"],
    ["W", "B"],
    ["U", "R"],
    ["B", "G"],
    ["R", "W"],
    ["G", "U"],
]
TRIS = [
    ["W", "U", "B"],
    ["U", "B", "R"],
    ["B", "R", "G"],
    ["R", "G", "W"],
    ["G", "W", "U"],
    ["W", "B", "G"],
    ["U", "R", "W"],
    ["B", "G", "U"],
    ["R", "W", "B"],
    ["G", "U", "R"],
]

# Core spell templates (target ~36-39 nonlands before MV-based land sizing tops up / trims to 60 exact)
ARCHETYPES = {
    "aggro": {
        "base_lands": 21,
        "land_band": (20, 22),
        "tapped_cap": 4,
        "c_lo": 19,
        "c_mid": 7,
        "c_hi": 0,
        "rem": 5,
        "burn": 5,
        "cnt": 0,
        "swp": 0,
        "pmp": 3,
        "drw": 0,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 5,
        "min_early": 14,
        "aggressive": True,
    },
    "gowide": {
        "base_lands": 22,
        "land_band": (20, 22),
        "tapped_cap": 4,
        "c_lo": 18,
        "c_mid": 9,
        "c_hi": 0,
        "rem": 4,
        "burn": 2,
        "cnt": 0,
        "swp": 0,
        "pmp": 4,
        "drw": 1,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 5,
        "min_early": 13,
        "aggressive": True,
    },
    "tempo": {
        "base_lands": 21,
        "land_band": (20, 22),
        "tapped_cap": 4,
        "c_lo": 13,
        "c_mid": 6,
        "c_hi": 0,
        "rem": 6,
        "burn": 3,
        "cnt": 6,
        "swp": 0,
        "pmp": 1,
        "drw": 4,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 5,
        "min_early": 13,
        "aggressive": True,
    },
    "midrange": {
        "base_lands": 24,
        "land_band": (23, 24),
        "tapped_cap": 8,
        "c_lo": 8,
        "c_mid": 12,
        "c_hi": 4,
        "rem": 7,
        "burn": 1,
        "cnt": 0,
        "swp": 0,
        "pmp": 0,
        "drw": 4,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 6,
        "min_early": 8,
        "aggressive": False,
    },
    "bigmana": {
        "base_lands": 25,
        "land_band": (25, 26),
        "tapped_cap": 8,
        "c_lo": 4,
        "c_mid": 9,
        "c_hi": 9,
        "rem": 5,
        "burn": 0,
        "cnt": 0,
        "swp": 1,
        "pmp": 0,
        "drw": 2,
        "rmp": 5,
        "copies": (4, 3, 2),
        "slice_mult": 6,
        "min_early": 7,
        "aggressive": False,
    },
    "control": {
        "base_lands": 26,
        "land_band": (25, 26),
        "tapped_cap": 8,
        "c_lo": 2,
        "c_mid": 5,
        "c_hi": 5,
        "rem": 9,
        "burn": 0,
        "cnt": 6,
        "swp": 3,
        "pmp": 0,
        "drw": 4,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 6,
        "min_early": 7,
        "aggressive": False,
    },
    "tribal": {
        "base_lands": 22,
        "land_band": (21, 23),
        "tapped_cap": 4,
        "c_lo": 16,
        "c_mid": 11,
        "c_hi": 2,
        "rem": 5,
        "burn": 1,
        "cnt": 0,
        "swp": 0,
        "pmp": 2,
        "drw": 1,
        "rmp": 0,
        "copies": (4, 3, 2),
        "slice_mult": 5,
        "min_early": 12,
        "aggressive": True,
    },
    "limited_aggro": {
        "base_lands": 23,
        "land_band": (22, 23),
        "tapped_cap": 4,
        "c_lo": 14,
        "c_mid": 11,
        "c_hi": 2,
        "rem": 5,
        "burn": 2,
        "cnt": 0,
        "swp": 0,
        "pmp": 3,
        "drw": 0,
        "rmp": 0,
        "copies": (2, 1, 1),
        "slice_mult": 8,
        "min_early": 10,
        "aggressive": True,
    },
    "limited_mid": {
        "base_lands": 24,
        "land_band": (23, 24),
        "tapped_cap": 6,
        "c_lo": 9,
        "c_mid": 13,
        "c_hi": 5,
        "rem": 5,
        "burn": 0,
        "cnt": 0,
        "swp": 0,
        "pmp": 2,
        "drw": 2,
        "rmp": 0,
        "copies": (2, 1, 1),
        "slice_mult": 8,
        "min_early": 8,
        "aggressive": False,
    },
    "limited_ctrl": {
        "base_lands": 25,
        "land_band": (24, 25),
        "tapped_cap": 7,
        "c_lo": 5,
        "c_mid": 10,
        "c_hi": 6,
        "rem": 7,
        "burn": 0,
        "cnt": 2,
        "swp": 2,
        "pmp": 0,
        "drw": 3,
        "rmp": 0,
        "copies": (2, 1, 1),
        "slice_mult": 8,
        "min_early": 7,
        "aggressive": False,
    },
    "brawl_aggro": {
        "base_lands": 24,
        "land_band": (24, 25),
        "tapped_cap": 4,
        "c_lo": 15,
        "c_mid": 11,
        "c_hi": 2,
        "rem": 4,
        "burn": 2,
        "cnt": 0,
        "swp": 0,
        "pmp": 1,
        "drw": 1,
        "rmp": 0,
        "copies": (1,),
        "slice_mult": 10,
        "min_early": 10,
        "aggressive": True,
    },
    "brawl_mid": {
        "base_lands": 24,
        "land_band": (24, 25),
        "tapped_cap": 8,
        "c_lo": 9,
        "c_mid": 12,
        "c_hi": 5,
        "rem": 6,
        "burn": 0,
        "cnt": 1,
        "swp": 1,
        "pmp": 0,
        "drw": 2,
        "rmp": 0,
        "copies": (1,),
        "slice_mult": 10,
        "min_early": 8,
        "aggressive": False,
    },
    "brawl_ramp": {
        "base_lands": 25,
        "land_band": (24, 25),
        "tapped_cap": 8,
        "c_lo": 5,
        "c_mid": 9,
        "c_hi": 9,
        "rem": 5,
        "burn": 0,
        "cnt": 0,
        "swp": 1,
        "pmp": 0,
        "drw": 2,
        "rmp": 4,
        "copies": (1,),
        "slice_mult": 10,
        "min_early": 7,
        "aggressive": False,
    },
    "brawl_ctrl": {
        "base_lands": 25,
        "land_band": (24, 25),
        "tapped_cap": 8,
        "c_lo": 4,
        "c_mid": 7,
        "c_hi": 6,
        "rem": 8,
        "burn": 0,
        "cnt": 4,
        "swp": 3,
        "pmp": 0,
        "drw": 3,
        "rmp": 0,
        "copies": (1,),
        "slice_mult": 10,
        "min_early": 7,
        "aggressive": False,
    },
}

TRIBAL_SPECS = [
    (["W"], "Human", 3),
    (["G", "W"], "Human", 3),
    (["R", "W"], "Human", 2),
    (["W"], "Soldier", 2),
    (["W", "U"], "Soldier", 2),
    (["U", "R"], "Wizard", 3),
    (["U"], "Wizard", 2),
    (["G"], "Elf", 3),
    (["B", "G"], "Elf", 2),
    (["B"], "Vampire", 2),
    (["W", "B"], "Vampire", 2),
    (["B", "R"], "Vampire", 2),
    (["B"], "Zombie", 2),
    (["U", "B"], "Zombie", 2),
    (["R"], "Goblin", 2),
    (["B", "R"], "Goblin", 1),
    (["W", "U"], "Spirit", 1),
    (["U", "B"], "Rogue", 1),
    (["R", "G"], "Warrior", 1),
]

MANA_SYM_RE = re.compile(r"\{([^}]+)\}")
REM_PAREN_RE = re.compile(r"\([^)]*\)")
WARD_COST_RE = re.compile(r"\bward\s+(?:\{[^}]+\})+", re.I)

DEAD_RULES_RE = re.compile(
    r"outside the game|command tower|your commander|choose a background|partner with|"
    r"\bante\b|conspiracy|contraption|attraction|sticker|"
    r"enchant player|enchant artifact|enchant enchantment|enchant planeswalker|"
    r"enchant land(?! you control.*add)|"
    r"target (?:white|blue|black|red|green|multicolored|monocolored|colorless|snow) |"
    r"protection from (?:white|blue|black|red|green)\b",
    re.I,
)

TRIBES_WATCH = (
    "human", "soldier", "wizard", "cleric", "rogue", "elf", "elves", "vampire",
    "zombie", "knight", "elemental", "bird", "spirit", "beast", "goblin",
    "merfolk", "dragon", "angel", "demon", "dinosaur", "sliver", "ally",
    "pirate", "cat", "dog", "rat", "faerie", "kithkin", "treefolk", "dwarf",
    "samurai", "ninja", "shaman", "druid", "berserker", "artificer",
)
TRIBE_PAYOFF_RE = re.compile(
    r"(?:other |target |each |whenever you cast an? |search your library for an? )("
    + "|".join(TRIBES_WATCH)
    + r")s?\b",
    re.I,
)


def fnv64(text: str) -> str:
    h = 0xCBF29CE484222325
    for b in text.encode("utf-8"):
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def parse_deck_cards(text: str) -> Counter:
    c = Counter()
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith("#") or s.startswith("//") or s.startswith("SB:"):
            continue
        parts = s.split(None, 1)
        if len(parts) == 2 and parts[0].isdigit():
            c[parts[1].strip()] += int(parts[0])
    return c


def jaccard_nonbasic(a: Counter, b: Counter) -> float:
    sa = {k for k in a if k not in ALL_BASIC_NAMES}
    sb = {k for k in b if k not in ALL_BASIC_NAMES}
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def parse_cost_pips(cost_syms):
    """Returns `(pips_dict, has_x, has_explicit_c, colors_set)` from a list/str of `{...}` symbols."""
    raw = "".join(cost_syms) if isinstance(cost_syms, list) else str(cost_syms or "")
    pips = {c: 0.0 for c in "WUBRG"}
    has_x = False
    has_c = False
    cols = set()
    for sym in MANA_SYM_RE.findall(raw.upper()):
        parts = sym.split("/")
        if len(parts) == 1:
            p = parts[0]
            if p in ("X", "Y", "Z"):
                has_x = True
            elif p == "C":
                has_c = True
            elif p in COLOR_SET:
                pips[p] += 1.0
                cols.add(p)
        else:
            c_parts = [p for p in parts if p in COLOR_SET]
            if "C" in parts:
                has_c = True
            for cp in c_parts:
                cols.add(cp)
                pips[cp] += 1.0 / max(1, len(c_parts))
    return pips, has_x, has_c, cols


def extract_rules_colors_and_c(view):
    """Extracts all `{W/U/B/R/G}` mana symbols in rules/back-face/split costs (excluding reminder text & Ward)."""
    chunks = list(view.get("rules") or []) + list(view.get("rightSplitRules") or [])
    bf = view.get("secondCardFace")
    if isinstance(bf, dict):
        chunks += list(bf.get("rules") or [])
    cleaned = WARD_COST_RE.sub("", REM_PAREN_RE.sub("", " ".join(chunks)))
    cols = set()
    has_act_c = False
    for line in cleaned.splitlines():
        # Check explicit `{C}` before `:` (activated cost requiring colorless)
        if ":" in line:
            lhs = line.split(":", 1)[0]
            for sym in MANA_SYM_RE.findall(lhs.upper()):
                if "C" in sym.split("/"):
                    has_act_c = True
        for sym in MANA_SYM_RE.findall(line.upper()):
            for p in sym.split("/"):
                if p in COLOR_SET:
                    cols.add(p)
    return cols, has_act_c


def enrich_card(c, view):
    """Enriches a `decktool --dump-catalog` entry with full-sheet cost, color identity, and tactical metadata."""
    v = view if isinstance(view, dict) else {}
    nm = c["name"]
    rules_all = " ".join(
        list(v.get("rules") or [])
        + list(v.get("rightSplitRules") or [])
        + (list(v["secondCardFace"].get("rules") or []) if isinstance(v.get("secondCardFace"), dict) else [])
    )
    rules_clean = REM_PAREN_RE.sub("", rules_all).lower()

    if c["is_land"]:
        tapped = (
            ("enters tapped" in rules_clean or "enters the battlefield tapped" in rules_clean)
            and ("unless" not in rules_clean)
            and ("pay 2 life" not in rules_clean)
            and ("pay 3 life" not in rules_clean)
        )
        return {
            **c,
            "enters_tapped": bool(tapped),
            "dead": bool(DEAD_RULES_RE.search(rules_clean) or "command" in nm.lower()),
        }

    left_syms = list(v.get("manaCostLeftStr") or [])
    if not left_syms:
        return {**c, "dead": True}

    pips_l, has_x, has_c_l, cols_l = parse_cost_pips(left_syms)
    _, _, has_c_r1, cols_r1 = parse_cost_pips(v.get("manaCostRightStr") or [])
    _, _, has_c_r2, cols_r2 = parse_cost_pips(v.get("rightSplitCostsStr") or "")
    _, _, _, cols_ci = parse_cost_pips(v.get("originalColorIdentity") or "")
    rules_cols, has_act_c = extract_rules_colors_and_c(v)

    full_ci = set(c["colors"]) | cols_l | cols_r1 | cols_r2 | cols_ci | rules_cols
    if has_c_l or has_c_r1 or has_c_r2 or has_act_c:
        return {**c, "dead": True}
    if DEAD_RULES_RE.search(rules_clean):
        return {**c, "dead": True}

    mv = int(c["mv"])
    if mv <= 0 or mv > 7:
        return {**c, "dead": True}

    is_defender = bool(
        "Wall" in (c.get("subtypes") or [])
        or re.search(r"\bdefender\b|can't attack\b", rules_clean)
    )
    is_ramp = bool(
        re.search(
            r"add \{[wubrg]\}|add one mana|search your library for a[^\n.]*land|create a[^\n.]*treasure token",
            rules_clean,
        )
    )
    is_tokens = bool(re.search(r"create (?:a|two|three|four|x|\d+) [^\n.]*creature token", rules_clean))
    is_draw = bool(
        c["draw"]
        or re.search(
            r"draw (?:a|two|three|x|\d+) card|exile the top [^\n.]*you may play|look at the top [^\n.]*into your hand",
            rules_clean,
        )
    )
    is_removal = bool(
        c["removal"]
        or re.search(
            r"destroy target (?:creature|permanent|nonland)|exile target (?:creature|permanent|nonland)|"
            r"deals \d+ damage to target creature|deals \d+ damage to any target|"
            r"target creature gets -\d+/-\d+|target opponent sacrifices a creature|"
            r"fights? target creature|return target (?:creature|nonland permanent) [^\n.]*to its owner's hand",
            rules_clean,
        )
    )
    is_burn = bool(c["face"] or re.search(r"deals \d+ damage to any target|deals \d+ damage to target player|each opponent loses \d+ life", rules_clean))
    is_equip_or_aura = bool("Equipment" in (c.get("subtypes") or []) or "Aura" in (c.get("subtypes") or []))

    # Detect narrow tribal payoff cards
    tribal_dep = None
    m_tr = TRIBE_PAYOFF_RE.search(rules_clean)
    if m_tr:
        tr_word = m_tr.group(1).lower()
        if tr_word == "elves":
            tr_word = "elf"
        # Self-contained token makers ("create two 1/1 red Goblin creature tokens") are fine anywhere
        if f"{tr_word} creature token" not in rules_clean:
            tribal_dep = tr_word.capitalize()

    if c["is_creature"]:
        p, t = int(c["power"]), int(c["toughness"])
        if p <= 0 and not is_ramp and not is_defender:
            return {**c, "dead": True}
        if t <= 0 and "enters with" not in rules_clean and "enters the battlefield with" not in rules_clean:
            return {**c, "dead": True}
        if has_x and mv <= 1 and "enters with x" not in rules_clean:
            return {**c, "dead": True}
    else:
        if has_x and mv <= 2 and not (is_burn or is_removal or c["counter"] or is_draw or is_tokens):
            return {**c, "dead": True}
        has_real_role = (
            is_removal
            or c["sweeper"]
            or is_burn
            or c["counter"]
            or is_draw
            or is_ramp
            or is_tokens
            or c["is_pw"]
            or c["pump"]
        )
        if not has_real_role:
            return {**c, "dead": True}
        # Reject overcosted clunky Equipment (`Equip {4}+`)
        if "Equipment" in (c.get("subtypes") or []) and re.search(r"equip \{[4-9]\}", rules_clean):
            return {**c, "dead": True}

    has_lifelink = "lifelink" in rules_clean
    has_deathtouch = "deathtouch" in rules_clean
    has_menace = "menace" in rules_clean
    has_vigilance = "vigilance" in rules_clean
    has_first_strike = "first strike" in rules_clean or "double strike" in rules_clean
    enters_tapped_drawback = "enters tapped" in rules_clean or "enters the battlefield tapped" in rules_clean

    return {
        **c,
        "dead": False,
        "full_ci": full_ci,
        "pips": pips_l,
        "has_x": has_x,
        "is_defender": is_defender,
        "is_ramp": is_ramp,
        "is_tokens": is_tokens,
        "is_draw": is_draw,
        "is_removal": is_removal,
        "is_burn": is_burn,
        "is_equip_or_aura": is_equip_or_aura,
        "tribal_dep": tribal_dep,
        "lifelink": has_lifelink,
        "deathtouch": has_deathtouch,
        "menace": has_menace,
        "vigilance": has_vigilance,
        "first_strike": has_first_strike,
        "etb_tapped": enters_tapped_drawback,
    }


def card_score(c, arch_name: str, focus_tribe=None) -> float:
    """Principled archetype-aware card quality score (eliminates MV<=3 non-creature chaff inflation)."""
    mv = max(1, int(c["mv"]))
    aggressive = ARCHETYPES[arch_name]["aggressive"]

    if c["is_creature"]:
        if c["is_defender"] and aggressive:
            return -20.0
        p, t = max(0, int(c["power"])), max(1, int(c["toughness"]))
        stat_rate = min(2.25, (1.15 * p + 0.85 * t) / max(1.25, float(mv)))
        bonus = 0.0
        if c["flying"]:
            bonus += 0.30
        if c["haste"]:
            bonus += 0.35 if aggressive else 0.15
        if c["flash"]:
            bonus += 0.25
        if c["menace"] or c["first_strike"]:
            bonus += 0.22
        if c["trample"] or c["lifelink"] or c["deathtouch"] or c["vigilance"]:
            bonus += 0.18
        if c["is_removal"] or c["is_draw"] or c["is_tokens"]:
            bonus += 0.42
        if c["is_ramp"] and not aggressive:
            bonus += 0.35
        if c["etb_tapped"]:
            bonus -= 0.35 if aggressive else 0.15
        if focus_tribe:
            if focus_tribe in (c.get("subtypes") or []):
                bonus += 0.65
            if c.get("tribal_dep") == focus_tribe:
                bonus += 0.85
        elif c.get("tribal_dep") is not None:
            # Penalize narrow tribal lords in non-tribal decks unless vanilla body is already elite
            bonus -= 0.75
        return stat_rate + bonus

    if focus_tribe and c.get("tribal_dep") == focus_tribe:
        tribe_bonus = 0.85
    elif c.get("tribal_dep") is not None:
        return -15.0
    else:
        tribe_bonus = 0.0

    s = 0.55 + tribe_bonus
    if c["is_pw"]:
        s += 1.35
    if c["sweeper"]:
        s += 1.35 if not aggressive else -1.0
    if c["is_removal"]:
        s += 1.15 + (0.30 if mv <= 2 else 0.0) + (0.22 if c["is_instant"] else 0.0)
    if c["counter"]:
        s += 1.10 + (0.30 if mv <= 2 else 0.0)
    if c["is_burn"]:
        s += 1.05 + (0.28 if mv <= 2 else 0.0) + (0.18 if c["is_instant"] else 0.0)
    if c["is_draw"]:
        s += 0.95 + (0.20 if mv <= 3 else 0.0) + (0.15 if c["is_instant"] else 0.0)
    if c["is_tokens"]:
        s += 0.98 + (0.25 if arch_name == "gowide" else 0.0)
    if c["is_ramp"]:
        s += (1.05 if ("bigmana" in arch_name or "ramp" in arch_name) else 0.45) + (0.25 if mv <= 2 else 0.0)
    if c["pump"] and not (c["is_removal"] or c["is_burn"] or c["is_draw"]):
        s += 0.75 if aggressive else 0.20
    if mv >= 6 and not (c["sweeper"] or c["is_pw"] or "bigmana" in arch_name or "ramp" in arch_name):
        s -= 0.45
    return s


def compute_target_lands(arch_name: str, picked: Counter, by_name: dict) -> int:
    spec = ARCHETYPES[arch_name]
    lo, hi = spec["land_band"]
    tot_spells = max(1, sum(picked.values()))
    avg_mv = sum(by_name[nm]["mv"] * cnt for nm, cnt in picked.items()) / float(tot_spells)
    if lo == 20 and hi == 22:
        if avg_mv < 1.92:
            return 20
        if avg_mv < 2.30:
            return 21
        return 22
    if lo == 21 and hi == 23:
        return 21 if avg_mv < 2.15 else (22 if avg_mv < 2.55 else 23)
    if lo == 22 and hi == 23:
        return 22 if avg_mv < 2.45 else 23
    if lo == 23 and hi == 24:
        return 23 if avg_mv < 2.90 else 24
    if lo == 24 and hi == 25:
        return 24 if avg_mv < 3.00 else 25
    if lo == 25 and hi == 26:
        return 25 if avg_mv < 3.35 else 26
    return spec["base_lands"]


def build_karsten_manabase(rng, colors, arch_name, n_lands, picked, by_name, dual_lands, used_counts):
    """Sizes duals and basics proportionally to exact colored pip demand, respecting tapped-dual caps."""
    spec = ARCHETYPES[arch_name]
    singleton = spec["copies"] == (1,)
    tapped_cap = spec["tapped_cap"]
    cset = set(colors)

    # Karsten weighted pip demand: early double/triple pips get 1.5x weight per extra pip
    pip_demand = {col: 1.5 for col in colors}
    for nm, cnt in picked.items():
        card = by_name[nm]
        mv = max(1, card["mv"])
        early_factor = 1.35 if mv <= 2 else (1.15 if mv == 3 else 1.0)
        for col in colors:
            p = card["pips"].get(col, 0.0)
            if p > 0:
                eff_p = p if p <= 1.0 else (1.0 + 1.5 * (p - 1.0))
                pip_demand[col] += cnt * eff_p * early_factor

    lands = Counter()
    sources = {col: 0.0 for col in colors}
    tapped_used = 0
    land_rem = n_lands

    if len(colors) >= 2:
        min_basics_total = len(colors) * 2
        if "limited" in arch_name:
            max_duals = 5 if len(colors) == 2 else 8
        elif singleton:
            max_duals = 9 if len(colors) == 2 else 13
        else:
            max_duals = 10 if len(colors) == 2 else 14

        elig_duals = [
            l for l in dual_lands if len(l["land_colors"]) >= 2 and set(l["land_colors"]).issubset(cset)
        ]

        def dual_priority(l):
            pair_w = sum(pip_demand.get(col, 0.0) for col in l["land_colors"])
            untapped_bonus = 0.0 if l["enters_tapped"] else 25.0
            reuse_pen = 0.4 * used_counts[l["name"]]
            return untapped_bonus + pair_w - reuse_pen + rng.uniform(-3.0, 3.0)

        elig_duals.sort(key=dual_priority, reverse=True)
        duals_added = 0
        for l in elig_duals:
            if duals_added >= max_duals or land_rem <= min_basics_total:
                break
            is_tap = l["enters_tapped"]
            if is_tap and tapped_used >= tapped_cap:
                continue
            cap_card = 1 if singleton else (2 if "limited" in arch_name else 4)
            k = min(cap_card, max_duals - duals_added, land_rem - min_basics_total)
            if is_tap:
                k = min(k, tapped_cap - tapped_used)
            if k <= 0:
                continue
            lands[l["name"]] += k
            duals_added += k
            land_rem -= k
            if is_tap:
                tapped_used += k
            for col in l["land_colors"]:
                if col in sources:
                    sources[col] += k

    # Guarantee >= 2 basics of every deck color first (when land_rem permits)
    for col in colors:
        if land_rem >= len(colors) * 2:
            lands[BASICS[col]] += 2
            sources[col] += 2.0
            land_rem -= 2
        elif land_rem > 0:
            lands[BASICS[col]] += 1
            sources[col] += 1.0
            land_rem -= 1

    # Greedily allocate remaining basic lands to close the largest Karsten source deficit
    tot_demand = sum(pip_demand.values())
    total_sources_goal = max(float(n_lands), sum(sources.values()) + land_rem)
    target_sources = {col: total_sources_goal * (pip_demand[col] / tot_demand) for col in colors}

    while land_rem > 0:
        best_col = max(colors, key=lambda c: (target_sources[c] - sources[c], pip_demand[c]))
        lands[BASICS[best_col]] += 1
        sources[best_col] += 1.0
        land_rem -= 1

    return lands, tapped_used, sources


def simulate_coherence_gate(rng_seed: int, arch_name: str, colors, picked: Counter, lands: Counter, by_name: dict) -> tuple:
    """Runs structural coherence assertions + a 200-hand London-mulligan turn-3 castability simulation."""
    spec = ARCHETYPES[arch_name]
    cset = set(colors)
    n_spells = sum(picked.values())
    n_lands = sum(lands.values())
    if n_spells + n_lands != 60:
        return False, f"size={n_spells + n_lands}"

    lo_b, hi_b = spec["land_band"]
    if not (lo_b <= n_lands <= hi_b):
        return False, f"lands={n_lands} outside {spec['land_band']}"

    tapped_cnt = sum(cnt for nm, cnt in lands.items() if nm in by_name and by_name[nm].get("enters_tapped"))
    if tapped_cnt > spec["tapped_cap"]:
        return False, f"tapped={tapped_cnt} > {spec['tapped_cap']}"

    early_plays = 0
    wincons = 0
    for nm, cnt in picked.items():
        c = by_name[nm]
        if not c["full_ci"].issubset(cset):
            return False, f"off_color={nm}"
        if c["mv"] <= 2 and (c["is_creature"] or c["is_removal"] or c["is_burn"] or c["is_ramp"] or c["is_draw"]):
            early_plays += cnt
        if (c["is_creature"] and c["power"] >= 2) or c["is_pw"] or c["is_tokens"]:
            wincons += cnt

    if early_plays < spec["min_early"]:
        return False, f"early={early_plays} < {spec['min_early']}"
    if wincons < 6:
        return False, f"wincons={wincons} < 6"

    # Build 60-card deck list for Monte Carlo opening-hand + 3-draw simulation (with 1 London mulligan on 0-1/6-7 land hands)
    deck_cards = []
    for nm, cnt in picked.items():
        deck_cards.extend([nm] * cnt)
    for nm, cnt in lands.items():
        deck_cards.extend([nm] * cnt)

    sim_rng = random.Random(rng_seed)
    n_trials = 200
    unplayable = 0

    def hand_playable_by_t3(nine_cards):
        land_cols_avail = Counter()
        total_lands = 0
        spells = []
        for nm in nine_cards:
            if nm in ALL_BASIC_NAMES:
                total_lands += 1
                for col, bnm in BASICS.items():
                    if nm == bnm:
                        land_cols_avail[col] += 1
            elif by_name[nm]["is_land"]:
                total_lands += 1
                for col in by_name[nm]["land_colors"]:
                    land_cols_avail[col] += 1
            else:
                spells.append(by_name[nm])
        if total_lands < 2 or not spells:
            return False
        mana_cap = min(total_lands, 3)
        for sp in spells:
            if sp["mv"] <= mana_cap:
                ok = True
                for col, req in sp["pips"].items():
                    if req > 0 and land_cols_avail[col] < int(req + 0.49):
                        ok = False
                        break
                if ok:
                    return True
        return False

    for _ in range(n_trials):
        sim_rng.shuffle(deck_cards)
        h7 = deck_cards[:7]
        l7 = sum(1 for nm in h7 if nm in ALL_BASIC_NAMES or by_name[nm]["is_land"])
        if 2 <= l7 <= 5:
            nine = deck_cards[:9]
        else:
            # London mulligan to 6 + 2 draws by turn 3 = 8 cards seen
            sim_rng.shuffle(deck_cards)
            nine = deck_cards[:8]
        if not hand_playable_by_t3(nine):
            unplayable += 1

    unplayable_rate = unplayable / float(n_trials)
    if unplayable_rate > 0.16:
        return False, f"unplayable_rate={unplayable_rate:.2f}"
    return True, f"ok(unplayable={unplayable_rate:.2f})"


def build_deck(rng, colors, arch_name, roles, dual_lands, used_counts, focus_tribe=None):
    cset = set(colors)
    spec = ARCHETYPES[arch_name]
    copies = spec["copies"]
    slice_mult = spec["slice_mult"]
    singleton = copies == (1,)
    aggressive = spec["aggressive"]
    by_name = roles["_by_name"]

    def eligible(pool, max_mv=7, min_mv=1, allow_colorless=True):
        out = []
        for c in pool:
            ci = c["full_ci"]
            if not ci.issubset(cset):
                continue
            if not ci and not allow_colorless:
                continue
            if c["mv"] < min_mv or c["mv"] > max_mv:
                continue
            if aggressive and c["is_creature"] and c["is_defender"]:
                continue
            if focus_tribe is None and c.get("tribal_dep") is not None:
                if not c["is_creature"]:
                    continue
                # Only allow orphan tribal creature if its vanilla stat rate is already strong
                if (c["power"] + c["toughness"]) < 2 * c["mv"]:
                    continue
            out.append(c)
        return out

    picked = Counter()
    colorless_used = 0
    equip_aura_used = 0

    def pick_cards(pool, slots, copy_choices=None):
        nonlocal colorless_used, equip_aura_used
        if slots <= 0 or not pool:
            return 0
        cc_use = copy_choices if (copy_choices is not None and not singleton) else copies
        cands = [c for c in pool if c["name"] not in picked]
        if not cands:
            return 0
        scored = []
        for c in cands:
            # Encourage wide exploration across the 12k+ quality-gated verified pool
            reuse_pen = 0.55 * used_counts[c["name"]]
            sc = card_score(c, arch_name, focus_tribe=focus_tribe) - reuse_pen + rng.uniform(-0.45, 0.45)
            scored.append((sc, c))
        scored.sort(key=lambda x: x[0], reverse=True)
        top = [c for _, c in scored[: max(36, slots * slice_mult)]]
        rng.shuffle(top)
        # Re-bias mildly toward higher score after shuffle so quality stays strong
        top.sort(key=lambda c: card_score(c, arch_name, focus_tribe=focus_tribe) - 0.35 * used_counts[c["name"]] + rng.uniform(-0.5, 0.5), reverse=True)
        added = 0
        for c in top:
            if added >= slots:
                break
            rem = slots - added
            if not c["full_ci"] and colorless_used >= 4:
                continue
            if c["is_equip_or_aura"] and equip_aura_used >= (3 if aggressive else 1):
                continue
            if singleton:
                cnt = 1
            elif c["supertypes"] and "Legendary" in c["supertypes"]:
                cnt = min(rem, rng.choice([1, 2]))
            else:
                cnt = min(rem, rng.choice(cc_use))
            if not c["full_ci"]:
                cnt = min(cnt, 4 - colorless_used)
            if c["is_equip_or_aura"]:
                cnt = min(cnt, (3 if aggressive else 1) - equip_aura_used)
            if cnt <= 0:
                continue
            picked[c["name"]] = cnt
            added += cnt
            if not c["full_ci"]:
                colorless_used += cnt
            if c["is_equip_or_aura"]:
                equip_aura_used += cnt
        return added

    c_lo = eligible(roles["c_lo"], max_mv=2, min_mv=1)
    c_mid = eligible(roles["c_mid"], max_mv=4, min_mv=3)
    c_hi = eligible(roles["c_hi"], max_mv=7, min_mv=5)
    rem_p = eligible(roles["removal"], max_mv=4 if aggressive else 5, min_mv=1)
    burn_p = eligible(roles["burn"], max_mv=4, min_mv=1)
    cnt_p = eligible(roles["counter"], max_mv=4, min_mv=1)
    swp_p = eligible(roles["sweeper"], max_mv=6, min_mv=3)
    pmp_p = eligible(roles["pump"], max_mv=3, min_mv=1)
    drw_p = eligible(roles["draw"], max_mv=5, min_mv=1)
    rmp_p = eligible(roles["ramp"], max_mv=3, min_mv=1)

    if focus_tribe:
        tr_lo = [c for c in c_lo if focus_tribe in (c.get("subtypes") or []) or c.get("tribal_dep") == focus_tribe]
        tr_mid = [c for c in c_mid if focus_tribe in (c.get("subtypes") or []) or c.get("tribal_dep") == focus_tribe]
        if len(tr_lo) >= 6:
            c_lo = tr_lo + [c for c in c_lo if c not in tr_lo]
        if len(tr_mid) >= 5:
            c_mid = tr_mid + [c for c in c_mid if c not in tr_mid]

    pick_cards(swp_p, spec["swp"])
    pick_cards(cnt_p, spec["cnt"])
    pick_cards(burn_p, spec["burn"])
    pick_cards(rem_p, spec["rem"])
    pick_cards(rmp_p, spec["rmp"])
    pick_cards(drw_p, spec["drw"])
    pick_cards(c_lo, spec["c_lo"])
    pick_cards(c_mid, spec["c_mid"])
    pick_cards(c_hi, spec["c_hi"])
    if sum(by_name[nm]["is_creature"] * cnt for nm, cnt in picked.items()) >= 14:
        pick_cards(pmp_p, spec["pmp"])

    # Compute exact MV-matched land target and adjust spell count to `60 - n_lands`
    n_lands = compute_target_lands(arch_name, picked, by_name)
    target_spells = 60 - n_lands
    cur = sum(picked.values())

    if cur < target_spells:
        if aggressive:
            fb = c_lo + c_mid + rem_p + burn_p
        elif "bigmana" in arch_name or "ramp" in arch_name:
            fb = rmp_p + c_mid + c_hi + rem_p + drw_p
        elif "control" in arch_name or "ctrl" in arch_name:
            fb = rem_p + cnt_p + drw_p + c_mid + c_hi
        else:
            fb = c_lo + c_mid + rem_p + drw_p + c_hi
        pick_cards(fb, target_spells - cur)

    cur = sum(picked.values())
    if cur < target_spells:
        max_any = 4 if aggressive else 6
        any_p = eligible(roles["any_spell"], max_mv=max_any, min_mv=1)
        pick_cards(any_p, target_spells - cur)

    # Recompute target lands after top-up and trim excess spells if `cur > target_spells`
    n_lands = compute_target_lands(arch_name, picked, by_name)
    target_spells = 60 - n_lands
    while sum(picked.values()) > target_spells:
        # Trim highest-MV or lowest-score non-essential spell by 1
        candidates_trim = sorted(
            picked.keys(),
            key=lambda nm: (card_score(by_name[nm], arch_name, focus_tribe=focus_tribe), -by_name[nm]["mv"]),
        )
        nm0 = candidates_trim[0]
        if picked[nm0] <= 1:
            del picked[nm0]
        else:
            picked[nm0] -= 1

    if sum(picked.values()) < target_spells:
        any_p = eligible(roles["any_spell"], max_mv=5, min_mv=1)
        pick_cards(any_p, target_spells - sum(picked.values()))

    lands, _, _ = build_karsten_manabase(
        rng, colors, arch_name, n_lands, picked, by_name, dual_lands, used_counts
    )
    return picked, lands


def build_deck_plan():
    """Constructs the deterministic 758-deck synthetic plan (418 Constructed + 180 Limited + 160 Brawl = 758)."""
    plan = []

    # 1. 60 Mono-color Constructed (5 x 12)
    mono_archs = {
        "W": ["aggro", "aggro", "aggro", "gowide", "gowide", "gowide", "midrange", "midrange", "midrange", "bigmana", "control", "control"],
        "U": ["tempo", "tempo", "tempo", "tempo", "midrange", "midrange", "midrange", "bigmana", "control", "control", "control", "control"],
        "B": ["aggro", "aggro", "aggro", "midrange", "midrange", "midrange", "midrange", "bigmana", "bigmana", "control", "control", "control"],
        "R": ["aggro", "aggro", "aggro", "aggro", "gowide", "gowide", "gowide", "midrange", "midrange", "midrange", "bigmana", "bigmana"],
        "G": ["aggro", "aggro", "gowide", "gowide", "gowide", "midrange", "midrange", "midrange", "bigmana", "bigmana", "bigmana", "bigmana"],
    }
    for cols in MONO:
        c = cols[0]
        for idx, arch in enumerate(mono_archs[c]):
            plan.append((cols, arch, None, f"mono-{c.lower()}-{arch}-{idx + 1:02d}"))

    # 2. 200 Guild Constructed (10 x 20)
    guild_archs = [
        "aggro", "aggro", "aggro", "aggro",
        "gowide", "gowide", "gowide",
        "tempo", "tempo", "tempo",
        "midrange", "midrange", "midrange", "midrange",
        "bigmana", "bigmana", "bigmana",
        "control", "control", "control",
    ]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, arch in enumerate(guild_archs):
            a = arch
            if a == "tempo" and "U" not in cols and "R" not in cols:
                a = "aggro" if idx % 2 == 0 else "midrange"
            if a == "control" and "U" not in cols and "W" not in cols and "B" not in cols:
                a = "midrange" if idx % 2 == 0 else "bigmana"
            plan.append((cols, a, None, f"guild-{tag}-{a}-{idx + 1:02d}"))

    # 3. 120 Tri-color Constructed (10 x 12)
    tri_archs = [
        "aggro", "aggro", "gowide", "tempo",
        "midrange", "midrange", "midrange", "midrange",
        "bigmana", "bigmana", "control", "control",
    ]
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, arch in enumerate(tri_archs):
            a = arch
            if a == "tempo" and "U" not in cols:
                a = "midrange"
            plan.append((cols, a, None, f"tri-{tag}-{a}-{idx + 1:02d}"))

    # 4. 38 Tribal Synergy Constructed decks
    for cols, tribe, count in TRIBAL_SPECS:
        tag = "".join(cols).lower()
        for idx in range(count):
            plan.append((cols, "tribal", tribe, f"tribal-{tag}-{tribe.lower()}-{idx + 1:02d}"))

    # 5. 180 Limited-curve low-copy decks (10 guilds x 12 + 10 tris x 6)
    lim_guild = [
        "limited_aggro", "limited_aggro", "limited_aggro", "limited_aggro",
        "limited_mid", "limited_mid", "limited_mid", "limited_mid", "limited_mid",
        "limited_ctrl", "limited_ctrl", "limited_ctrl",
    ]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(lim_guild):
            plan.append((cols, a, None, f"lim-g-{tag}-{a.split('_')[1]}-{idx + 1:02d}"))

    lim_tri = ["limited_aggro", "limited_aggro", "limited_mid", "limited_mid", "limited_ctrl", "limited_ctrl"]
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(lim_tri):
            plan.append((cols, a, None, f"lim-t-{tag}-{a.split('_')[1]}-{idx + 1:02d}"))

    # 6. 160 Brawl/Singleton 60-card decks (5 mono x 4 + 10 guilds x 8 + 10 tris x 6)
    brawl_mono = ["brawl_aggro", "brawl_mid", "brawl_ramp", "brawl_ctrl"]
    for cols in MONO:
        c = cols[0]
        for idx, a in enumerate(brawl_mono):
            eff = "brawl_mid" if (a == "brawl_ctrl" and c in ("R", "G")) else a
            plan.append((cols, eff, None, f"brawl-m-{c.lower()}-{eff.split('_')[1]}-{idx + 1:02d}"))

    brawl_guild = [
        "brawl_aggro", "brawl_aggro",
        "brawl_mid", "brawl_mid", "brawl_mid",
        "brawl_ramp", "brawl_ctrl", "brawl_ctrl",
    ]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(brawl_guild):
            eff = "brawl_mid" if (a == "brawl_ctrl" and "U" not in cols and "W" not in cols and "B" not in cols) else a
            plan.append((cols, eff, None, f"brawl-g-{tag}-{eff.split('_')[1]}-{idx + 1:02d}"))

    brawl_tri = ["brawl_aggro", "brawl_mid", "brawl_mid", "brawl_ramp", "brawl_ramp", "brawl_ctrl"]
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(brawl_tri):
            plan.append((cols, a, None, f"brawl-t-{tag}-{a.split('_')[1]}-{idx + 1:02d}"))

    assert len(plan) == 758, f"expected 758 synthetic decks, got {len(plan)}"
    return plan


def load_or_dump_catalog(root: Path, catalog_arg: str):
    if catalog_arg and os.path.exists(catalog_arg):
        return [json.loads(line) for line in open(catalog_arg, encoding="utf-8")]
    decktool = root / "rust/target/release/decktool"
    if not decktool.exists():
        raise FileNotFoundError(f"decktool binary not found at {decktool} and --catalog not provided")
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        tmp_path = tf.name
    try:
        subprocess.run([str(decktool), "--dump-catalog", tmp_path], check=True)
        return [json.loads(line) for line in open(tmp_path, encoding="utf-8")]
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)


def run_engine_verification(root: Path):
    decktool = root / "rust/target/release/decktool"
    archbench = root / "rust/target/release/archbench"
    suite_dir = root / "archbench/constructed-train-800"

    if decktool.exists():
        t0 = time.time()
        proc = subprocess.run(
            [str(decktool), "--check-career", "--root", str(root)],
            capture_output=True,
            text=True,
            check=True,
        )
        miss_t800 = []
        ok_t800 = 0
        for line in proc.stdout.splitlines():
            if "Mage.Server/career-src/train-800/" in line:
                if line.startswith("OK\t"):
                    ok_t800 += 1
                else:
                    miss_t800.append(line)
        print(
            f"DECKTOOL CHECK: train-800 ok={ok_t800}, missing={len(miss_t800)} "
            f"({proc.stderr.strip()}) in {time.time() - t0:.2f}s",
            flush=True,
        )
        if miss_t800 or ok_t800 != 758:
            raise RuntimeError(f"decktool --check-career failed on train-800: ok={ok_t800}, miss={miss_t800[:5]}")

    if archbench.exists():
        t1 = time.time()
        p_s6 = subprocess.run(
            [
                str(archbench),
                "--suite", str(suite_dir),
                "--entrants", "host@s6,rust-quick",
                "--deals", "12",
                "--jobs", "8",
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        dt_s6 = time.time() - t1
        failed_s6 = [ln for ln in p_s6.stdout.splitlines() if ": failed" in ln or "DECKS CHANGED" in ln]
        print(f"ARCHBENCH host@s6 vs rust-quick (12 deals / 24 games, {dt_s6:.2f}s):\n{p_s6.stdout.strip()}", flush=True)
        if failed_s6:
            raise RuntimeError(f"archbench host@s6 smoke failed: {failed_s6[:5]}")

        t2 = time.time()
        p_wide = subprocess.run(
            [
                str(archbench),
                "--suite", str(suite_dir),
                "--entrants", "rust-quick,random",
                "--deals", "400",
                "--jobs", "8",
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        dt_wide = time.time() - t2
        failed_wide = [ln for ln in p_wide.stdout.splitlines() if ": failed" in ln or "DECKS CHANGED" in ln]
        print(f"ARCHBENCH rust-quick vs random (400 deals / 800 games, {dt_wide:.2f}s):\n{p_wide.stdout.strip()}", flush=True)
        if failed_wide:
            raise RuntimeError(f"archbench 400-deal sweep failed: {failed_wide[:5]}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(Path(__file__).resolve().parents[2]))
    ap.add_argument("--catalog", default="")
    ap.add_argument("--seed", type=int, default=202607)
    ap.add_argument("--skip-engine-check", action="store_true")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    rng = random.Random(args.seed)

    # 1. Load held-out OOD suite and collect EVERY OOD non-basic card for 0.0% synthetic OOD leakage
    ood_suite = json.loads((root / "archbench/constructed-ood-1/suite.json").read_text(encoding="utf-8"))
    ood_files = {d["file"] for d in ood_suite["decks"]}
    ood_hashes = {d["fnv64"] for d in ood_suite["decks"]}
    ood_bags = []
    ood_nonbasic_cards = set()
    for d in ood_suite["decks"]:
        p = root / d["file"]
        bag = parse_deck_cards(p.read_text(encoding="utf-8", errors="replace"))
        ood_bags.append(bag)
        ood_nonbasic_cards.update(k for k in bag if k not in ALL_BASIC_NAMES)

    # 2. Load 42 curated Pioneer/Standard decks from constructed-2 (untouched)
    c2_suite = json.loads((root / "archbench/constructed-2/suite.json").read_text(encoding="utf-8"))
    c2_entries = list(c2_suite["decks"])
    c2_cards = set()
    for d in c2_entries:
        bag = parse_deck_cards((root / d["file"]).read_text(encoding="utf-8", errors="replace"))
        c2_cards.update(bag.keys())

    # 3. Load frozen 16,483 verified card registry + views.json.gz + CardDb catalog
    reg_path = root / "rust/crates/playmat-ai/src/card_registry_32k.txt"
    verified_set = {ln.strip() for ln in reg_path.read_text(encoding="utf-8").splitlines() if ln.strip()}
    with gzip.open(root / "rust/crates/playmat-engine/cards/views.json.gz", "rt", encoding="utf-8") as f:
        raw_views = json.load(f)

    raw_catalog = load_or_dump_catalog(root, args.catalog)
    dual_lands = []
    roles = defaultdict(list)
    by_name = {}

    for raw_c in raw_catalog:
        nm = raw_c["name"]
        if nm in ALL_BASIC_NAMES:
            by_name[nm] = {**raw_c, "enters_tapped": False, "dead": False}
            continue
        # Restrict strictly to verified 16,483 registry MINUS all 447 OOD non-basic cards
        if nm not in verified_set or nm in ood_nonbasic_cards or "//" in nm:
            continue
        c = enrich_card(raw_c, raw_views.get(nm))
        if c.get("dead"):
            continue
        by_name[nm] = c
        if c["is_land"]:
            if not c["is_basic"] and len(c["land_colors"]) >= 2:
                dual_lands.append(c)
            continue
        roles["any_spell"].append(c)
        if c["is_creature"]:
            if c["mv"] <= 2 and (c["power"] >= 1 or c["is_ramp"]):
                roles["c_lo"].append(c)
            elif 3 <= c["mv"] <= 4 and c["power"] >= 2:
                roles["c_mid"].append(c)
            elif 5 <= c["mv"] <= 7 and c["power"] >= 3:
                roles["c_hi"].append(c)
        if c["is_removal"] and not c["sweeper"]:
            roles["removal"].append(c)
        if c["sweeper"]:
            roles["sweeper"].append(c)
        if c["is_burn"]:
            roles["burn"].append(c)
        if c["counter"]:
            roles["counter"].append(c)
        if c["pump"] and not c["is_removal"]:
            roles["pump"].append(c)
        if c["is_draw"]:
            roles["draw"].append(c)
        if c["is_ramp"]:
            roles["ramp"].append(c)

    roles["_by_name"] = by_name
    print(
        "CATALOG POOLS (verified & OOD-disjoint):",
        {k: len(v) for k, v in roles.items() if k != "_by_name"},
        f"duals={len(dual_lands)} (untapped_cond={sum(not d['enters_tapped'] for d in dual_lands)}, tapped={sum(d['enters_tapped'] for d in dual_lands)})",
        flush=True,
    )

    plan = build_deck_plan()
    out_dir = root / "Mage.Server/career-src/train-800"
    out_dir.mkdir(parents=True, exist_ok=True)
    for old in out_dir.glob("*.txt"):
        old.unlink()

    used_counts = Counter()
    synth_entries = []
    seen_hashes = set(ood_hashes) | {d["fnv64"] for d in c2_entries}
    max_synth_ood_jacc = 0.0
    synth_cards = set()
    gate_rejects = Counter()

    for idx_d, (cols, arch, tribe, slug) in enumerate(plan):
        built = False
        for attempt in range(60):
            picked, lands = build_deck(rng, cols, arch, roles, dual_lands, used_counts, focus_tribe=tribe)
            ok, reason = simulate_coherence_gate(
                args.seed * 10000 + idx_d * 100 + attempt, arch, cols, picked, lands, by_name
            )
            if not ok:
                gate_rejects[reason.split("=")[0]] += 1
                continue
            bag = Counter(picked) + Counter(lands)
            # Verify 0 non-basic OOD card leakage and 0.000 OOD Jaccard on every synthetic deck
            leaked = (set(bag.keys()) - ALL_BASIC_NAMES) & ood_nonbasic_cards
            if leaked:
                raise AssertionError(f"OOD card leaked into {slug}: {leaked}")
            mj = max((jaccard_nonbasic(bag, ob) for ob in ood_bags), default=0.0)
            lines = [
                f"# name: Train800 {slug}",
                f"# archetype: {arch}" + (f" ({tribe})" if tribe else ""),
                f"# colors: {''.join(cols)}",
                "# source: synthetic-train-800 (strictly OOD-card-disjoint, Karsten manabase)",
            ]
            for nm, cnt in sorted(picked.items()):
                lines.append(f"{cnt} {nm}")
            for nm, cnt in sorted(lands.items()):
                lines.append(f"{cnt} {nm}")
            text = "\n".join(lines) + "\n"
            h = fnv64(text)
            if h not in seen_hashes and mj == 0.0:
                seen_hashes.add(h)
                max_synth_ood_jacc = max(max_synth_ood_jacc, mj)
                for nm in picked:
                    used_counts[nm] += 1
                for nm in lands:
                    if nm not in ALL_BASIC_NAMES:
                        used_counts[nm] += 1
                synth_cards.update(picked.keys())
                synth_cards.update(lands.keys())
                rel = f"Mage.Server/career-src/train-800/{slug}.txt"
                (root / rel).write_text(text, encoding="utf-8")
                synth_entries.append({"file": rel, "fnv64": h})
                built = True
                break
        if not built:
            raise RuntimeError(f"Failed quality gate after 60 attempts for {slug} ({cols}, {arch}, {tribe})")

    all_decks = c2_entries + synth_entries
    for d in all_decks:
        assert d["file"] not in ood_files, f"OOD filename leak: {d['file']}"
        assert d["fnv64"] not in ood_hashes, f"OOD hash leak: {d['fnv64']}"

    all_distinct_cards = c2_cards | synth_cards
    distinct_verified = len(all_distinct_cards & verified_set)
    synth_ood_overlap = len((synth_cards - ALL_BASIC_NAMES) & ood_nonbasic_cards)
    assert synth_ood_overlap == 0, f"expected 0 synthetic OOD card overlap, got {synth_ood_overlap}"
    assert max_synth_ood_jacc <= 0.08, f"expected max_synth_ood_jacc <= 0.08, got {max_synth_ood_jacc}"
    assert distinct_verified >= 7500, f"expected >= 7500 distinct verified cards, got {distinct_verified}"

    suite_dir = root / "archbench/constructed-train-800"
    suite_dir.mkdir(parents=True, exist_ok=True)
    n = len(all_decks)
    suite = {
        "suite": "constructed-train@800",
        "created": "2026-11-28",
        "about": (
            f"Phase 7 quality-gated training deck pool: 42 constructed-2 Pioneer/Standard decks + "
            f"{len(synth_entries)} Karsten-manabase 60-card Constructed, Tribal, Limited-curve, and "
            f"Brawl/Singleton decks across all mono/2-color/3-color identities ({distinct_verified} distinct "
            f"verified cards; 0 synthetic OOD card overlap, max synthetic OOD Jaccard {max_synth_ood_jacc:.3f})."
        ),
        "deals": n * (n - 1),
        "turn_cap": 30,
        "duplicate": True,
        "anchor": {"random": 1000},
        "decks": all_decks,
        "mulligans": True,
    }
    suite_path = suite_dir / "suite.json"
    suite_path.write_text(json.dumps(suite, indent=2) + "\n", encoding="utf-8")

    print(
        f"BUILT constructed-train-800: total={n} decks (42 constructed-2 + {len(synth_entries)} synth), "
        f"distinct_verified_cards={distinct_verified} (synth_distinct={len(synth_cards)}), "
        f"synth_ood_nonbasic_overlap={synth_ood_overlap}, max_synth_ood_jaccard={max_synth_ood_jacc:.4f}, "
        f"mulligans={suite['mulligans']}, gate_rejects_resampled={dict(gate_rejects)}",
        flush=True,
    )

    if not args.skip_engine_check:
        run_engine_verification(root)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate and verify a 220-deck Constructed training suite (`archbench/constructed-train-200`)
strictly disjoint from `archbench/constructed-ood-1`.

Combines the 42 curated `constructed-2` Pioneer/Standard decks with 178 deterministic,
archetype-structured 60-card decks built from the full 14,024-card `playmat-engine` CardDb
across all 5 mono-colors, 10 two-color guilds, and 10 three-color combinations.
"""
import argparse, json, os, random
from collections import Counter, defaultdict
from pathlib import Path

BASICS = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}
MONO = [["W"], ["U"], ["B"], ["R"], ["G"]]
GUILDS = [
    ["W", "U"], ["U", "B"], ["B", "R"], ["R", "G"], ["G", "W"],
    ["W", "B"], ["U", "R"], ["B", "G"], ["R", "W"], ["G", "U"],
]
TRIS = [
    ["W", "U", "B"], ["U", "B", "R"], ["B", "R", "G"], ["R", "G", "W"], ["G", "W", "U"],
    ["W", "B", "G"], ["U", "R", "W"], ["B", "G", "U"], ["R", "W", "B"], ["G", "U", "R"],
]

# Archetype templates: (lands, target counts by role: creatures_low, creatures_mid, creatures_hi,
# removal, burn, counter, sweeper, pump, draw)
ARCHETYPES = {
    "aggro":   {"lands": 21, "c_lo": 20, "c_mid": 6,  "c_hi": 0,  "rem": 4,  "burn": 6, "cnt": 0, "swp": 0, "pmp": 3, "drw": 0},
    "gowide":  {"lands": 22, "c_lo": 22, "c_mid": 8,  "c_hi": 0,  "rem": 4,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 4, "drw": 0},
    "tempo":   {"lands": 22, "c_lo": 12, "c_mid": 6,  "c_hi": 0,  "rem": 6,  "burn": 2, "cnt": 6, "swp": 0, "pmp": 2, "drw": 4},
    "midrange":{"lands": 24, "c_lo": 8,  "c_mid": 12, "c_hi": 4,  "rem": 8,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 0, "drw": 4},
    "bigmana": {"lands": 25, "c_lo": 6,  "c_mid": 10, "c_hi": 11, "rem": 6,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 2, "drw": 0},
    "control": {"lands": 26, "c_lo": 0,  "c_mid": 4,  "c_hi": 4,  "rem": 10, "burn": 0, "cnt": 8, "swp": 4, "pmp": 0, "drw": 4},
}


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
    sa = {k for k in a if k not in BASICS.values()}
    sb = {k for k in b if k not in BASICS.values()}
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def card_score(c):
    """Heuristic constructed playability score so synthetic decks feel like real MTG."""
    mv = max(1, c["mv"])
    if c["is_creature"]:
        p, t = c["power"], c["toughness"]
        if p <= 0 and t <= 0:
            return -10.0
        eff = (p + t) / (2.0 * mv)
        bonus = 0.0
        if c["haste"]:
            bonus += 0.35
        if c["flying"]:
            bonus += 0.30
        if c["flash"]:
            bonus += 0.25
        if c["trample"]:
            bonus += 0.20
        return eff + bonus
    s = 1.0
    if c["is_instant"]:
        s += 0.4
    if c["removal"] or c["counter"] or c["sweeper"] or c["face"]:
        s += 0.6
    if mv <= 3:
        s += 0.4
    elif mv >= 6:
        s -= 0.4
    return s


def build_deck(rng, colors, arch_name, catalog_by_role, dual_lands, used_counts):
    cset = set(colors)
    spec = dict(ARCHETYPES[arch_name])
    n_lands = spec["lands"]
    n_spells = 60 - n_lands

    def eligible(pool, max_mv=7, min_mv=1):
        out = []
        for c in pool:
            cc = set(c["colors"])
            if not cc.issubset(cset):
                continue
            if len(cc) == 0 and c["mv"] == 0:
                continue
            if c["mv"] < min_mv or c["mv"] > max_mv:
                continue
            out.append(c)
        return out

    def pick_cards(pool, slots, copy_choices=(4, 3, 2)):
        nonlocal picked
        if slots <= 0 or not pool:
            return 0
        # Weight toward high playability + under-used cards for wide catalog coverage
        cands = [c for c in pool if c["name"] not in picked]
        if not cands:
            return 0
        # Sort by quality + slight diversity jitter, pick from top slice
        scored = []
        for c in cands:
            reuse_pen = 0.12 * used_counts[c["name"]]
            sc = card_score(c) - reuse_pen + rng.uniform(-0.45, 0.45)
            scored.append((sc, c))
        scored.sort(key=lambda x: x[0], reverse=True)
        top = [c for _, c in scored[:max(20, slots * 4)]]
        rng.shuffle(top)
        added = 0
        for c in top:
            if added >= slots:
                break
            rem = slots - added
            if c["supertypes"] and "Legendary" in c["supertypes"]:
                cnt = min(rem, rng.choice([2, 3]))
            else:
                cnt = min(rem, rng.choice(copy_choices))
            if cnt <= 0:
                continue
            picked[c["name"]] = cnt
            added += cnt
        return added

    picked = {}
    # Role-specific pools filtered to color identity
    c_lo = eligible(catalog_by_role["c_lo"], max_mv=2, min_mv=1)
    c_mid = eligible(catalog_by_role["c_mid"], max_mv=4, min_mv=3)
    c_hi = eligible(catalog_by_role["c_hi"], max_mv=7, min_mv=5)
    rem_p = eligible(catalog_by_role["removal"], max_mv=5, min_mv=1)
    burn_p = eligible(catalog_by_role["burn"], max_mv=4, min_mv=1)
    cnt_p = eligible(catalog_by_role["counter"], max_mv=4, min_mv=1)
    swp_p = eligible(catalog_by_role["sweeper"], max_mv=6, min_mv=2)
    pmp_p = eligible(catalog_by_role["pump"], max_mv=3, min_mv=1)
    drw_p = eligible(catalog_by_role["draw"], max_mv=5, min_mv=1)
    any_nonland = eligible(catalog_by_role["any_spell"], max_mv=5 if arch_name != "bigmana" else 7, min_mv=1)

    pick_cards(swp_p, spec["swp"], (2, 3, 4))
    pick_cards(cnt_p, spec["cnt"], (3, 4, 2))
    pick_cards(burn_p, spec["burn"], (4, 3))
    pick_cards(rem_p, spec["rem"], (3, 4, 2))
    pick_cards(pmp_p, spec["pmp"], (3, 4, 2))
    pick_cards(drw_p, spec["drw"], (3, 4, 2))
    pick_cards(c_lo, spec["c_lo"], (4, 3))
    pick_cards(c_mid, spec["c_mid"], (4, 3, 2))
    pick_cards(c_hi, spec["c_hi"], (3, 2, 4))

    # Fill any remaining nonland slots cleanly from archetype-matched pool
    cur = sum(picked.values())
    if cur < n_spells:
        fallback = c_lo + c_mid if arch_name in ("aggro", "gowide", "tempo") else (c_mid + c_hi + rem_p if arch_name == "bigmana" else rem_p + drw_p + c_mid + any_nonland)
        pick_cards(fallback, n_spells - cur, (4, 3, 2))
    cur = sum(picked.values())
    if cur < n_spells:
        pick_cards(any_nonland, n_spells - cur, (4, 3, 2))

    # Color pip proportions for manabase
    col_weight = Counter({c: 1 for c in colors})
    by_name = catalog_by_role["_by_name"]
    for nm, cnt in picked.items():
        for col in by_name[nm]["colors"]:
            if col in cset:
                col_weight[col] += cnt

    lands = Counter()
    land_rem = n_lands
    # Add dual/multi lands matching subset of deck colors (len >= 2)
    if len(colors) >= 2:
        good_duals = [
            l for l in dual_lands
            if len(l["land_colors"]) >= 2 and set(l["land_colors"]).issubset(cset)
        ]
        rng.shuffle(good_duals)
        max_duals = 8 if len(colors) == 2 else 12
        d_added = 0
        for l in good_duals:
            if d_added >= max_duals or land_rem <= len(colors) * 2:
                break
            k = min(4, max_duals - d_added, land_rem - len(colors) * 2)
            if k <= 0:
                break
            lands[l["name"]] = k
            d_added += k
            land_rem -= k

    # Allocate remaining lands as basics proportional to color weights
    tot_w = sum(col_weight[c] for c in colors)
    alloc = {}
    used_b = 0
    for i, col in enumerate(colors):
        if i == len(colors) - 1:
            n_b = land_rem - used_b
        else:
            n_b = max(1, round(land_rem * col_weight[col] / tot_w))
            n_b = min(n_b, land_rem - used_b - (len(colors) - 1 - i))
        alloc[col] = n_b
        used_b += n_b
    for col, n_b in alloc.items():
        if n_b > 0:
            lands[BASICS[col]] += n_b

    assert sum(picked.values()) + sum(lands.values()) == 60, (sum(picked.values()), sum(lands.values()))
    for nm in picked:
        used_counts[nm] += 1
    return picked, lands


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="/home/playmat/mage")
    ap.add_argument("--catalog", default="/tmp/card_catalog.jsonl")
    ap.add_argument("--target-synth", type=int, default=178)
    ap.add_argument("--seed", type=int, default=202605)
    args = ap.parse_args()

    root = Path(args.root)
    rng = random.Random(args.seed)

    # Load strictly held-out OOD decks for zero-leakage assertions
    ood_suite = json.loads((root / "archbench/constructed-ood-1/suite.json").read_text())
    ood_files = {d["file"] for d in ood_suite["decks"]}
    ood_hashes = {d["fnv64"] for d in ood_suite["decks"]}
    ood_bags = []
    for d in ood_suite["decks"]:
        p = root / d["file"]
        if p.exists():
            ood_bags.append(parse_deck_cards(p.read_text(errors="replace")))

    # Load all 42 constructed-2 decks
    c2_suite = json.loads((root / "archbench/constructed-2/suite.json").read_text())
    c2_entries = list(c2_suite["decks"])

    # Load CardDb catalog
    catalog = [json.loads(line) for line in open(args.catalog)]
    by_name = {c["name"]: c for c in catalog}

    # Filter clean playable non-land cards & dual lands
    dual_lands = []
    roles = defaultdict(list)
    roles["_by_name"] = by_name
    for c in catalog:
        nm = c["name"]
        if "//" in nm or c["mv"] > 7:
            continue
        if c["is_land"]:
            if not c["is_basic"] and len(c["land_colors"]) >= 2:
                dual_lands.append(c)
            continue
        if c["mv"] == 0:
            continue
        # Require reasonable constructed card (not vanilla 0/0 or weird un-card)
        if c["is_creature"] and (c["power"] <= 0 and c["toughness"] <= 0):
            continue
        roles["any_spell"].append(c)
        if c["is_creature"]:
            if c["mv"] <= 2 and c["power"] >= 1:
                roles["c_lo"].append(c)
            elif 3 <= c["mv"] <= 4 and c["power"] >= 2:
                roles["c_mid"].append(c)
            elif 5 <= c["mv"] <= 7 and c["power"] >= 4:
                roles["c_hi"].append(c)
        if c["removal"] and not c["sweeper"]:
            roles["removal"].append(c)
        if c["sweeper"]:
            roles["sweeper"].append(c)
        if c["face"]:
            roles["burn"].append(c)
        if c["counter"]:
            roles["counter"].append(c)
        if c["pump"]:
            roles["pump"].append(c)
        if c["draw"]:
            roles["draw"].append(c)

    print("CATALOG POOLS:", {k: len(v) for k, v in roles.items() if k != "_by_name"}, "duals:", len(dual_lands))

    # Build deterministic schedule of 178 diverse decks:
    # 25 mono (5x5), 100 two-color guilds (10x10), 53 three-color shards/wedges
    plan = []
    mono_archs = {
        "W": ["aggro", "gowide", "midrange", "bigmana", "control"],
        "U": ["tempo", "tempo", "midrange", "control", "control"],
        "B": ["aggro", "midrange", "midrange", "bigmana", "control"],
        "R": ["aggro", "aggro", "gowide", "midrange", "bigmana"],
        "G": ["aggro", "gowide", "midrange", "bigmana", "bigmana"],
    }
    for cols in MONO:
        for idx, arch in enumerate(mono_archs[cols[0]]):
            plan.append((cols, arch, f"mono-{cols[0].lower()}-{arch}-{idx+1}"))

    guild_archs = ["aggro", "gowide", "tempo", "midrange", "midrange", "bigmana", "bigmana", "control", "control", "tempo"]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, arch in enumerate(guild_archs):
            # Adjust archetypes slightly to color identity strengths
            a = arch
            if a == "tempo" and "U" not in cols and "R" not in cols:
                a = "aggro" if idx < 5 else "gowide"
            if a == "control" and "U" not in cols and "W" not in cols and "B" not in cols:
                a = "midrange" if idx % 2 == 0 else "bigmana"
            plan.append((cols, a, f"guild-{tag}-{a}-{idx+1}"))

    tri_archs = ["midrange", "control", "bigmana", "aggro", "gowide", "tempo"]
    tri_idx = 0
    while len(plan) < args.target_synth:
        cols = TRIS[tri_idx % len(TRIS)]
        a = tri_archs[(tri_idx // len(TRIS)) % len(tri_archs)]
        tag = "".join(cols).lower()
        plan.append((cols, a, f"tri-{tag}-{a}-{tri_idx+1}"))
        tri_idx += 1

    out_dir = root / "Mage.Server/career-src/train-200"
    out_dir.mkdir(parents=True, exist_ok=True)
    for old in out_dir.glob("*.txt"):
        old.unlink()

    used_counts = Counter()
    synth_entries = []
    seen_hashes = set(ood_hashes) | {d["fnv64"] for d in c2_entries}
    max_ood_jacc = 0.0
    unique_cards = set()

    for cols, arch, slug in plan:
        for attempt in range(20):
            picked, lands = build_deck(rng, cols, arch, roles, dual_lands, used_counts)
            bag = Counter(picked) + Counter(lands)
            mj = max((jaccard_nonbasic(bag, ob) for ob in ood_bags), default=0.0)
            lines = [
                f"# name: Train200 {slug}",
                f"# archetype: {arch}",
                f"# colors: {''.join(cols)}",
                "# source: synthetic-train-200 (disjoint from constructed-ood-1)",
            ]
            for nm, cnt in sorted(picked.items()):
                lines.append(f"{cnt} {nm}")
            for nm, cnt in sorted(lands.items()):
                lines.append(f"{cnt} {nm}")
            text = "\n".join(lines) + "\n"
            h = fnv64(text)
            if h not in seen_hashes and mj < 0.45:
                seen_hashes.add(h)
                max_ood_jacc = max(max_ood_jacc, mj)
                unique_cards.update(picked.keys())
                unique_cards.update(lands.keys())
                rel = f"Mage.Server/career-src/train-200/{slug}.txt"
                (root / rel).write_text(text)
                synth_entries.append({"file": rel, "fnv64": h})
                break
        else:
            raise RuntimeError(f"Failed to build unique disjoint deck for {slug}")

    all_decks = c2_entries + synth_entries
    # Final strict zero-leakage check
    for d in all_decks:
        assert d["file"] not in ood_files, f"OOD filename leak: {d['file']}"
        assert d["fnv64"] not in ood_hashes, f"OOD hash leak: {d['fnv64']}"

    suite_dir = root / "archbench/constructed-train-200"
    suite_dir.mkdir(parents=True, exist_ok=True)
    n = len(all_decks)
    suite = {
        "suite": "constructed-train@200",
        "created": "2026-11-26",
        "about": (
            f"Phase 5 expanded training deck pool: 42 constructed-2 Pioneer/Standard decks + "
            f"{len(synth_entries)} archetype-structured 60-card decks across all mono/2-color/3-color "
            f"identities ({len(unique_cards)} distinct cards used). Strictly disjoint from constructed-ood-1."
        ),
        "deals": n * (n - 1),
        "turn_cap": 30,
        "duplicate": True,
        "anchor": {"random": 1000},
        "decks": all_decks,
    }
    (suite_dir / "suite.json").write_text(json.dumps(suite, indent=2) + "\n")
    print(
        f"BUILT constructed-train-200: total={len(all_decks)} decks (42 real + {len(synth_entries)} synth), "
        f"unique_synth_cards={len(unique_cards)}, max_ood_jaccard={max_ood_jacc:.3f}, zero_ood_overlap=True"
    )


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate and verify a 452-deck Phase 6 training suite (`archbench/constructed-train-450`)
strictly disjoint from `archbench/constructed-ood-1`.

Combines the 42 curated `constructed-2` Pioneer/Standard decks with 410 deterministic,
archetype-structured 60-card decks built from the full 14,024-card `playmat-engine` CardDb:
  - 240 Constructed-style decks (aggro, gowide, tempo, midrange, bigmana, control)
  -  90 Limited-style mid-curve low-copy decks (1-2 copies, combat tricks + mid-curve)
  -  80 Brawl/Singleton-style 60-card decks (1-of nonlands across 2-3 colors, wide card tail)
Designed to expose CardEmb's BGE-small text projection to 4,000+ distinct implemented cards
while enforcing strict zero overlap and low Jaccard similarity against `constructed-ood-1`.
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

ARCHETYPES = {
    "aggro":         {"lands": 21, "c_lo": 20, "c_mid": 6,  "c_hi": 0,  "rem": 4,  "burn": 6, "cnt": 0, "swp": 0, "pmp": 3, "drw": 0, "copies": (4, 3, 2), "slice_mult": 4},
    "gowide":        {"lands": 22, "c_lo": 22, "c_mid": 8,  "c_hi": 0,  "rem": 4,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 4, "drw": 0, "copies": (4, 3, 2), "slice_mult": 4},
    "tempo":         {"lands": 22, "c_lo": 12, "c_mid": 6,  "c_hi": 0,  "rem": 6,  "burn": 2, "cnt": 6, "swp": 0, "pmp": 2, "drw": 4, "copies": (4, 3, 2), "slice_mult": 4},
    "midrange":      {"lands": 24, "c_lo": 8,  "c_mid": 12, "c_hi": 4,  "rem": 8,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 0, "drw": 4, "copies": (4, 3, 2), "slice_mult": 4},
    "bigmana":       {"lands": 25, "c_lo": 6,  "c_mid": 10, "c_hi": 11, "rem": 6,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 2, "drw": 0, "copies": (4, 3, 2), "slice_mult": 4},
    "control":       {"lands": 26, "c_lo": 0,  "c_mid": 4,  "c_hi": 4,  "rem": 10, "burn": 0, "cnt": 8, "swp": 4, "pmp": 0, "drw": 4, "copies": (4, 3, 2), "slice_mult": 4},
    "limited_aggro": {"lands": 23, "c_lo": 14, "c_mid": 12, "c_hi": 2,  "rem": 4,  "burn": 2, "cnt": 0, "swp": 0, "pmp": 3, "drw": 0, "copies": (1, 2, 1), "slice_mult": 8},
    "limited_mid":   {"lands": 24, "c_lo": 8,  "c_mid": 14, "c_hi": 5,  "rem": 5,  "burn": 0, "cnt": 0, "swp": 0, "pmp": 2, "drw": 2, "copies": (1, 2, 1), "slice_mult": 8},
    "limited_ctrl":  {"lands": 25, "c_lo": 4,  "c_mid": 10, "c_hi": 6,  "rem": 7,  "burn": 0, "cnt": 3, "swp": 2, "pmp": 0, "drw": 3, "copies": (1, 2, 1), "slice_mult": 8},
    "brawl_aggro":   {"lands": 23, "c_lo": 15, "c_mid": 11, "c_hi": 2,  "rem": 4,  "burn": 2, "cnt": 0, "swp": 0, "pmp": 2, "drw": 1, "copies": (1,),      "slice_mult": 10},
    "brawl_mid":     {"lands": 24, "c_lo": 8,  "c_mid": 12, "c_hi": 6,  "rem": 6,  "burn": 0, "cnt": 1, "swp": 1, "pmp": 0, "drw": 2, "copies": (1,),      "slice_mult": 10},
    "brawl_ramp":    {"lands": 25, "c_lo": 6,  "c_mid": 10, "c_hi": 11, "rem": 5,  "burn": 0, "cnt": 0, "swp": 1, "pmp": 0, "drw": 2, "copies": (1,),      "slice_mult": 10},
    "brawl_ctrl":    {"lands": 26, "c_lo": 2,  "c_mid": 8,  "c_hi": 6,  "rem": 8,  "burn": 0, "cnt": 4, "swp": 3, "pmp": 0, "drw": 3, "copies": (1,),      "slice_mult": 10},
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
    mv = max(1, c["mv"])
    if c["is_creature"]:
        p, t = c["power"], c["toughness"]
        if p <= 0 and t <= 0:
            return -10.0
        eff = (p + t) / (2.0 * mv)
        bonus = 0.0
        if c["haste"]: bonus += 0.35
        if c["flying"]: bonus += 0.30
        if c["flash"]: bonus += 0.25
        if c["trample"]: bonus += 0.20
        return eff + bonus
    s = 1.0
    if c["is_instant"]: s += 0.4
    if c["removal"] or c["counter"] or c["sweeper"] or c["face"]: s += 0.6
    if mv <= 3: s += 0.4
    elif mv >= 6: s -= 0.3
    return s


def build_deck(rng, colors, arch_name, catalog_by_role, dual_lands, used_counts):
    cset = set(colors)
    spec = dict(ARCHETYPES[arch_name])
    n_lands = spec["lands"]
    n_spells = 60 - n_lands
    copies = spec["copies"]
    slice_mult = spec["slice_mult"]
    singleton = (copies == (1,))

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

    def pick_cards(pool, slots, copy_choices=None):
        nonlocal picked
        if slots <= 0 or not pool:
            return 0
        cc_use = copy_choices if (copy_choices is not None and not singleton) else copies
        cands = [c for c in pool if c["name"] not in picked]
        if not cands:
            return 0
        scored = []
        for c in cands:
            reuse_pen = 0.35 * used_counts[c["name"]]
            sc = card_score(c) - reuse_pen + rng.uniform(-0.65, 0.65)
            scored.append((sc, c))
        scored.sort(key=lambda x: x[0], reverse=True)
        top = [c for _, c in scored[:max(32, slots * slice_mult)]]
        rng.shuffle(top)
        added = 0
        for c in top:
            if added >= slots:
                break
            rem = slots - added
            if singleton:
                cnt = 1
            elif c["supertypes"] and "Legendary" in c["supertypes"]:
                cnt = min(rem, rng.choice([1, 2]))
            else:
                cnt = min(rem, rng.choice(cc_use))
            if cnt <= 0:
                continue
            picked[c["name"]] = cnt
            added += cnt
        return added

    picked = {}
    c_lo = eligible(catalog_by_role["c_lo"], max_mv=2, min_mv=1)
    c_mid = eligible(catalog_by_role["c_mid"], max_mv=4, min_mv=3)
    c_hi = eligible(catalog_by_role["c_hi"], max_mv=7, min_mv=5)
    rem_p = eligible(catalog_by_role["removal"], max_mv=5, min_mv=1)
    burn_p = eligible(catalog_by_role["burn"], max_mv=4, min_mv=1)
    cnt_p = eligible(catalog_by_role["counter"], max_mv=4, min_mv=1)
    swp_p = eligible(catalog_by_role["sweeper"], max_mv=6, min_mv=2)
    pmp_p = eligible(catalog_by_role["pump"], max_mv=3, min_mv=1)
    drw_p = eligible(catalog_by_role["draw"], max_mv=5, min_mv=1)
    max_any_mv = 7 if ("bigmana" in arch_name or "brawl" in arch_name or "limited" in arch_name) else 5
    any_nonland = eligible(catalog_by_role["any_spell"], max_mv=max_any_mv, min_mv=1)

    pick_cards(swp_p, spec["swp"])
    pick_cards(cnt_p, spec["cnt"])
    pick_cards(burn_p, spec["burn"])
    pick_cards(rem_p, spec["rem"])
    pick_cards(pmp_p, spec["pmp"])
    pick_cards(drw_p, spec["drw"])
    pick_cards(c_lo, spec["c_lo"])
    pick_cards(c_mid, spec["c_mid"])
    pick_cards(c_hi, spec["c_hi"])

    cur = sum(picked.values())
    if cur < n_spells:
        if any(k in arch_name for k in ("aggro", "gowide", "tempo")):
            fallback = c_lo + c_mid + any_nonland
        elif "bigmana" in arch_name or "ramp" in arch_name:
            fallback = c_mid + c_hi + rem_p + any_nonland
        else:
            fallback = rem_p + drw_p + c_mid + c_hi + any_nonland
        pick_cards(fallback, n_spells - cur)
    cur = sum(picked.values())
    if cur < n_spells:
        pick_cards(any_nonland, n_spells - cur)

    col_weight = Counter({c: 1 for c in colors})
    by_name = catalog_by_role["_by_name"]
    for nm, cnt in picked.items():
        for col in by_name[nm]["colors"]:
            if col in cset:
                col_weight[col] += cnt

    lands = Counter()
    land_rem = n_lands
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
            cap = 1 if singleton else 4
            k = min(cap, max_duals - d_added, land_rem - len(colors) * 2)
            if k <= 0:
                break
            lands[l["name"]] = k
            d_added += k
            land_rem -= k

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
    ap.add_argument("--root", default="/home/brandonalamgarcianeri/playmat")
    ap.add_argument("--catalog", default="/tmp/card_catalog.jsonl")
    ap.add_argument("--seed", type=int, default=202606)
    args = ap.parse_args()

    root = Path(args.root)
    rng = random.Random(args.seed)

    ood_suite = json.loads((root / "archbench/constructed-ood-1/suite.json").read_text())
    ood_files = {d["file"] for d in ood_suite["decks"]}
    ood_hashes = {d["fnv64"] for d in ood_suite["decks"]}
    ood_bags = []
    for d in ood_suite["decks"]:
        p = root / d["file"]
        if p.exists():
            ood_bags.append(parse_deck_cards(p.read_text(errors="replace")))

    c2_suite = json.loads((root / "archbench/constructed-2/suite.json").read_text())
    c2_entries = list(c2_suite["decks"])

    catalog = [json.loads(line) for line in open(args.catalog)]
    by_name = {c["name"]: c for c in catalog}

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

    plan = []
    # 1. 40 Mono Constructed (5 x 8)
    mono_archs = {
        "W": ["aggro", "aggro", "gowide", "gowide", "midrange", "midrange", "bigmana", "control"],
        "U": ["tempo", "tempo", "tempo", "midrange", "midrange", "control", "control", "control"],
        "B": ["aggro", "aggro", "midrange", "midrange", "midrange", "bigmana", "control", "control"],
        "R": ["aggro", "aggro", "aggro", "gowide", "gowide", "midrange", "midrange", "bigmana"],
        "G": ["aggro", "gowide", "gowide", "midrange", "midrange", "bigmana", "bigmana", "bigmana"],
    }
    for cols in MONO:
        for idx, arch in enumerate(mono_archs[cols[0]]):
            plan.append((cols, arch, f"mono-{cols[0].lower()}-{arch}-{idx+1}"))

    # 2. 120 Guild Constructed (10 x 12)
    guild_archs = ["aggro", "aggro", "gowide", "gowide", "tempo", "tempo", "midrange", "midrange", "bigmana", "bigmana", "control", "control"]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, arch in enumerate(guild_archs):
            a = arch
            if a == "tempo" and "U" not in cols and "R" not in cols:
                a = "aggro" if idx % 2 == 0 else "gowide"
            if a == "control" and "U" not in cols and "W" not in cols and "B" not in cols:
                a = "midrange" if idx % 2 == 0 else "bigmana"
            plan.append((cols, a, f"guild-{tag}-{a}-{idx+1}"))

    # 3. 80 Tri-color Constructed (10 x 8)
    tri_archs = ["midrange", "midrange", "control", "control", "bigmana", "bigmana", "aggro", "gowide"]
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(tri_archs):
            plan.append((cols, a, f"tri-{tag}-{a}-{idx+1}"))

    # 4. 90 Limited-style mid-curve low-copy decks (10 guilds x 6 + 10 tris x 3)
    lim_guild = ["limited_aggro", "limited_aggro", "limited_mid", "limited_mid", "limited_ctrl", "limited_ctrl"]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(lim_guild):
            plan.append((cols, a, f"lim-g-{tag}-{a.split('_')[1]}-{idx+1}"))
    lim_tri = ["limited_aggro", "limited_mid", "limited_ctrl"]
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(lim_tri):
            plan.append((cols, a, f"lim-t-{tag}-{a.split('_')[1]}-{idx+1}"))

    # 5. 80 Brawl/Singleton 60-card decks (10 guilds x 4 + 10 tris x 4)
    brawl_archs = ["brawl_aggro", "brawl_mid", "brawl_ramp", "brawl_ctrl"]
    for cols in GUILDS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(brawl_archs):
            plan.append((cols, a, f"brawl-g-{tag}-{a.split('_')[1]}-{idx+1}"))
    for cols in TRIS:
        tag = "".join(cols).lower()
        for idx, a in enumerate(brawl_archs):
            plan.append((cols, a, f"brawl-t-{tag}-{a.split('_')[1]}-{idx+1}"))

    assert len(plan) == 410, len(plan)

    out_dir = root / "Mage.Server/career-src/train-450"
    out_dir.mkdir(parents=True, exist_ok=True)
    for old in out_dir.glob("*.txt"):
        old.unlink()

    used_counts = Counter()
    synth_entries = []
    seen_hashes = set(ood_hashes) | {d["fnv64"] for d in c2_entries}
    max_ood_jacc = 0.0
    unique_cards = set()

    for cols, arch, slug in plan:
        for attempt in range(25):
            picked, lands = build_deck(rng, cols, arch, roles, dual_lands, used_counts)
            bag = Counter(picked) + Counter(lands)
            mj = max((jaccard_nonbasic(bag, ob) for ob in ood_bags), default=0.0)
            lines = [
                f"# name: Train450 {slug}",
                f"# archetype: {arch}",
                f"# colors: {''.join(cols)}",
                "# source: synthetic-train-450 (disjoint from constructed-ood-1)",
            ]
            for nm, cnt in sorted(picked.items()):
                lines.append(f"{cnt} {nm}")
            for nm, cnt in sorted(lands.items()):
                lines.append(f"{cnt} {nm}")
            text = "\n".join(lines) + "\n"
            h = fnv64(text)
            if h not in seen_hashes and mj < 0.25:
                seen_hashes.add(h)
                max_ood_jacc = max(max_ood_jacc, mj)
                unique_cards.update(picked.keys())
                unique_cards.update(lands.keys())
                rel = f"Mage.Server/career-src/train-450/{slug}.txt"
                (root / rel).write_text(text)
                synth_entries.append({"file": rel, "fnv64": h})
                break
        else:
            raise RuntimeError(f"Failed to build unique disjoint deck for {slug}")

    all_decks = c2_entries + synth_entries
    for d in all_decks:
        assert d["file"] not in ood_files, f"OOD filename leak: {d['file']}"
        assert d["fnv64"] not in ood_hashes, f"OOD hash leak: {d['fnv64']}"

    suite_dir = root / "archbench/constructed-train-450"
    suite_dir.mkdir(parents=True, exist_ok=True)
    n = len(all_decks)
    suite = {
        "suite": "constructed-train@450",
        "created": "2026-11-27",
        "about": (
            f"Phase 6 expanded training deck pool: 42 constructed-2 Pioneer/Standard decks + "
            f"{len(synth_entries)} diverse 60-card Constructed, Limited-curve, and Brawl/Singleton decks "
            f"across all mono/2-color/3-color identities ({len(unique_cards)} distinct cards used). "
            f"Strictly disjoint from constructed-ood-1 (max Jaccard {max_ood_jacc:.3f})."
        ),
        "deals": n * (n - 1),
        "turn_cap": 30,
        "duplicate": True,
        "anchor": {"random": 1000},
        "decks": all_decks,
    }
    (suite_dir / "suite.json").write_text(json.dumps(suite, indent=2) + "\n")
    print(
        f"BUILT constructed-train-450: total={len(all_decks)} decks (42 real + {len(synth_entries)} synth), "
        f"unique_synth_cards={len(unique_cards)}, max_ood_jaccard={max_ood_jacc:.3f}, zero_ood_overlap=True"
    )


if __name__ == "__main__":
    main()

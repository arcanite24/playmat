#!/usr/bin/env python3
"""The ladder's bot roster (#430 + Gen 2 Playbooks overhaul):
Combines Career school decks (Gen 1, preserved with their measured ratings) and the dedicated
ladder deck pool under resources/ladder/decks/{standard,pioneer,modern}/ (Gen 2, piloted with
Forge-inspired AI Playbooks and wider search budgets).

    python3 Mage.Server/ladder-src/roster.py

Writes Mage.Server/src/main/resources/ladder/bots.json. Run `ladder-calibrate --fixed` after adding
Gen 2 bots to calibrate only the new bots against the existing scale.
"""
import hashlib
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.dirname(HERE)
REPO = os.path.dirname(SERVER)
RESOURCES = os.path.join(SERVER, "src", "main", "resources")
DECKS = os.path.join(RESOURCES, "career", "campaigns", "decks")
LADDER_DECKS = os.path.join(RESOURCES, "ladder", "decks")
PLAYBOOKS_PATH = os.path.join(RESOURCES, "ladder", "playbooks.json")
VIEWS_PATH = os.path.join(REPO, "rust", "crates", "playmat-engine", "cards", "views.json")
OUT = os.path.join(RESOURCES, "ladder", "bots.json")
FORMATS = ["standard", "pioneer", "modern"]
LADDER_COPIES = {"modern": ["open"]}

SKILLS = {1: "quick", 3: "light", 5: "half", 7: "full"}
# Gen 1 bands (school decks)
BANDS = {
    "starter": [1, 3, 5],
    "signature": [1, 3, 5],
    "act1": [1, 3, 5],
    "act2": [3, 5, 7],
    "act3": [5, 7],
    "meta": [5, 7],
    "budget": [5, 7],
}
# Gen 2 bands for dedicated ladder decks: avoid 3x skill cloning in the top band
LADDER_BANDS = {
    "rogue": [5],
    "tier2": [5, 7],
    "tier1": [7],
}
QUEUES = {("budget", 5): ["league"], ("budget", 7): ["league"], ("meta", 5): ["open"]}

GEN1_SKILLS = {
    1: "quick",
    3: "budget=1200+width=5+extra_turns=0+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=1+plans=0+mulligan_sim=0+blunder=100+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256",
    5: "budget=3000+width=8+extra_turns=0+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=2+plans=0+mulligan_sim=0+blunder=20+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256",
    7: "budget=6000+width=12+extra_turns=1+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=2+plans=0+mulligan_sim=0+blunder=0+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256",
}

# Gen 2 base search strings (playbook=<id> appended per bot): stronger high-rank search
GEN2_SKILLS = {
    5: "budget=4500+width=10+extra_turns=1+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=2+plans=0+mulligan_sim=0+blunder=10+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256",
    7: "budget=10000+width=16+extra_turns=1+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=2+plans=0+mulligan_sim=0+blunder=0+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256",
}

RETIRED = set()
PRIOR_DECK = {
    "starter": 0, "signature": 0, "act1": 50, "act2": 150, "act3": 250,
    "meta": 300, "budget": 200, "rogue": 180, "tier2": 260, "tier1": 360,
}
PRIOR_SKILL = {1: 0, 3: 150, 5: 250, 7: 380}
RARITIES = {1: ["common"], 3: ["common", "uncommon"], 5: ["uncommon", "rare"], 7: ["rare", "mythic"]}

GIVEN = [
    "Vessa", "Brann", "Oriel", "Tamsin", "Corwin", "Ilse", "Maren", "Dorian", "Selka", "Hollis", "Ysolde", "Fenwick",
    "Aldra", "Caspian", "Neve", "Rook", "Liora", "Thane", "Mirela", "Osric", "Wynn", "Kesta", "Garrick", "Petra",
    "Soren", "Elodie", "Bastian", "Rhea", "Lucan", "Sabine", "Torvald", "Inka", "Emrys", "Junia", "Kael", "Nadira",
    "Perrin", "Odile", "Hask", "Zara", "Calder", "Brielle", "Ansel", "Mireille", "Tobiah", "Fiora", "Edric", "Lark",
    "Quill", "Saoirse", "Varo", "Delphine", "Roan", "Talia", "Ivo", "Marisol", "Cato", "Wren", "Ezra", "Lysandra",
]
FAMILY = [
    "Quillmoor", "Ashgrove", "Vantreth", "Coldwater", "Ember", "Halloway", "Stormcall", "Duskmere", "Brightwell",
    "Thornfield", "Ravensmark", "Ironvale", "Saltmarsh", "Greaves", "Marrowind", "Fairhollow", "Blackwood", "Kestrel",
    "Oakhart", "Mistral", "Copperfield", "Wyndham", "Starling", "Holloway", "Redcliff", "Frostmere", "Larkspur",
    "Penhallow", "Grimsby", "Silverthorn",
]


def kind_of(stem):
    for k in ("starter", "signature", "meta", "budget", "act1", "act2", "act3", "tier1", "tier2", "rogue"):
        if stem.startswith(k):
            return k
    return None


def deck_name(path):
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("NAME:"):
                return line[5:].strip()
    return os.path.splitext(os.path.basename(path))[0]


def deck_colors(path, views):
    """True WUBRG color identity from all maindeck spells (via views.json) and basic lands."""
    basics = {"Plains": "W", "Island": "U", "Swamp": "B", "Mountain": "R", "Forest": "G"}
    color_map = {"white": "W", "blue": "U", "black": "B", "red": "R", "green": "G"}
    found = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            s = line.strip()
            if not s or s.startswith("NAME:") or s.startswith("SB:"):
                continue
            m = re.match(r"\d+\s+(?:\[[^\]]+\]\s+)?(.+)$", s)
            if not m:
                continue
            card = m.group(1).strip()
            if card in basics:
                found.add(basics[card])
            elif views and card in views:
                col = views[card].get("color", {})
                for k, letter in color_map.items():
                    if col.get(k):
                        found.add(letter)
    return "".join(c for c in "WUBRG" if c in found)


def infer_archetype(stem, name):
    text = f"{stem} {name}".lower()
    if "burn" in text:
        return "burn"
    if "control" in text:
        return "control"
    if "sacrifice" in text or "aristocrat" in text:
        return "sacrifice"
    if any(k in text for k in ("reanimat", "goryo", "grave", "phoenix", "greasefang")):
        return "graveyard"
    if any(k in text for k in ("combo", "storm", "mill", "rhinos", "living-end")):
        return "combo"
    if any(k in text for k in ("ramp", "landfall", "devotion", "domain")):
        return "ramp"
    if any(k in text for k in ("tempo", "prowess", "spirits", "skies", "merfolk", "rogue", "spellementals", "flash", "enchantments")):
        return "tempo"
    if any(k in text for k in ("aggro", "weenie", "mono-red", "red line", "red deck wins", "zoo", "lizards", "auras", "heroic", "convoke", "ensoul")):
        return "aggro"
    if any(k in text for k in ("humans", "vampires", "angels", "dragons", "elves", "goblins", "lifegain", "taxes")):
        return "tribal"
    return "midrange"


def tier_of(kind):
    if kind in ("tier1", "meta", "act3"):
        return "meta"
    if kind in ("tier2", "act2"):
        return "tier2"
    if kind == "rogue":
        return "rogue"
    if kind == "budget":
        return "budget"
    return "starter"


def h(text):
    return int(hashlib.sha256(text.encode()).hexdigest(), 16)


def main():
    cosmetics = json.load(open(os.path.join(RESOURCES, "career", "cosmetics.json"), encoding="utf-8"))
    playbooks = json.load(open(PLAYBOOKS_PATH, encoding="utf-8")) if os.path.exists(PLAYBOOKS_PATH) else {"decks": {}}
    pb_decks = playbooks.get("decks", {})
    views = json.load(open(VIEWS_PATH, encoding="utf-8")) if os.path.exists(VIEWS_PATH) else {}

    pool = {}
    for item in cosmetics:
        if item["kind"] in ("avatar", "sleeve", "playmat", "frame", "title") and not item.get("rank") and not item.get("mastery") and not item.get("ladder"):
            pool.setdefault(item["kind"], []).append(item)

    old = {}
    previous = {}
    if os.path.exists(OUT):
        previous = json.load(open(OUT, encoding="utf-8"))
        old = {b["id"]: b for b in previous.get("bots", [])}
    used_names = {b["persona"]["name"] for b in old.values()}

    def pick_cosmetic(kind, skill, seed, archetype=""):
        items = [i for i in pool[kind] if i.get("rarity") in RARITIES[skill]] or pool[kind]
        items.sort(key=lambda i: i["id"])
        return items[h(seed + kind + archetype) % len(items)]["id"]

    def name_for(seed):
        n = h(seed)
        for k in range(len(GIVEN) * len(FAMILY)):
            name = GIVEN[(n + k) % len(GIVEN)] + " " + FAMILY[(n // len(GIVEN) + k * 7) % len(FAMILY)]
            if name not in used_names:
                used_names.add(name)
                return name
        raise SystemExit("ran out of persona names")

    bots = []

    def add_bot(bot_id, fmt, deck_ref, path, kind, skill, gen, ai_str, playbook_id, queues=None):
        dname = deck_name(path)
        pb_meta = pb_decks.get(playbook_id, {}) if playbook_id else {}
        arch = pb_meta.get("archetype") or infer_archetype(os.path.splitext(os.path.basename(path))[0], dname)
        tier = pb_meta.get("tier") or tier_of(kind)
        before = old.get(bot_id)
        persona = before["persona"] if before else {
            "name": name_for(bot_id),
            "avatar": pick_cosmetic("avatar", skill, bot_id, arch),
            "sleeve": pick_cosmetic("sleeve", skill, bot_id, arch),
            "playmat": pick_cosmetic("playmat", skill, bot_id, arch),
            "frame": pick_cosmetic("frame", skill, bot_id, arch),
            "title": pick_cosmetic("title", skill, bot_id, arch),
        }
        prior = 1000 + PRIOR_DECK.get(kind, 150) + PRIOR_SKILL[skill] + (40 if gen >= 2 else 0)
        bot = {
            "id": bot_id,
            "format": fmt,
            "deck": deck_ref,
            "deckName": dname,
            "colors": deck_colors(path, views),
            "kind": kind,
            "archetype": arch,
            "tier": tier,
            "skill": skill,
            "search": SKILLS[skill],
            "ai": ai_str,
            "generation": gen,
            "prior": prior,
            "persona": persona,
        }
        if playbook_id:
            bot["playbook"] = playbook_id
        if pb_meta.get("tagline"):
            bot["tagline"] = pb_meta["tagline"]
        if pb_meta.get("keyCards"):
            bot["keyCards"] = pb_meta["keyCards"]
        if queues:
            bot["queues"] = queues
        if bot_id in RETIRED:
            bot["retired"] = True
        for key in ("rating", "rd", "games", "won", "drew", "lost"):
            if before and key in before:
                bot[key] = before[key]
        bots.append(bot)

    for fmt in FORMATS:
        # 1. Existing Career school decks (Gen 1 preserved + Gen 2 playbook upgrades for flagship meta decks)
        folder = os.path.join(DECKS, fmt)
        for file in sorted(os.listdir(folder)):
            stem, ext = os.path.splitext(file)
            if fmt in LADDER_COPIES:
                if not stem.startswith("ladder-"):
                    continue
                clean_stem = stem[len("ladder-"):]
            elif stem.startswith("ladder-"):
                continue
            else:
                clean_stem = stem
            kind = kind_of(clean_stem)
            if ext != ".dck" or kind is None:
                continue
            path = os.path.join(folder, file)
            deck_ref = f"{fmt}/{file}"
            pb_key = f"{fmt}/{stem}"
            queues_for_fmt = LADDER_COPIES.get(fmt)

            # Gen 1 bots (exact original IDs and pinned settings)
            for skill in BANDS.get(kind, []):
                if skill not in GEN1_SKILLS:
                    continue
                bot_id = f"{fmt}-{clean_stem}-s{skill}"
                q = queues_for_fmt if queues_for_fmt else QUEUES.get((kind, skill))
                add_bot(bot_id, fmt, deck_ref, path, kind, skill, 1, GEN1_SKILLS[skill], None, q)

            # Gen 2 upgrade for school meta decks that have a custom Playbook (Master tier s7)
            if kind == "meta" and pb_key in pb_decks:
                bot_id = f"{fmt}-{clean_stem}-s7-g2"
                ai_str = GEN2_SKILLS[7] + f"+playbook={pb_key}"
                q = queues_for_fmt if queues_for_fmt else ["open"]
                add_bot(bot_id, fmt, deck_ref, path, kind, 7, 2, ai_str, pb_key, q)

        # 2. Dedicated Ranked deck pool under resources/ladder/decks/<fmt>/ (Gen 2 Playbook bots)
        ladder_folder = os.path.join(LADDER_DECKS, fmt)
        if os.path.isdir(ladder_folder):
            for file in sorted(os.listdir(ladder_folder)):
                stem, ext = os.path.splitext(file)
                kind = kind_of(stem)
                if ext != ".dck" or kind is None:
                    continue
                path = os.path.join(ladder_folder, file)
                deck_ref = f"ladder/{fmt}/{file}"
                pb_key = f"ladder/{fmt}/{stem}"
                arch = pb_decks.get(pb_key, {}).get("archetype") or infer_archetype(stem, deck_name(path))
                for skill in LADDER_BANDS.get(kind, [5]):
                    # Skill 5 rogue/tier2 uses archetype profile; skill 7 uses full deck playbook
                    chosen_pb = pb_key if (skill >= 7 or pb_key in pb_decks) else f"profile:{arch}"
                    if skill == 5 and kind == "tier2":
                        chosen_pb = f"profile:{arch}"
                    bot_id = f"{fmt}-l-{stem}-s{skill}-g2"
                    ai_str = GEN2_SKILLS[skill] + f"+playbook={chosen_pb}"
                    queues = ["open"] if fmt == "modern" else None
                    add_bot(bot_id, fmt, deck_ref, path, kind, skill, 2, ai_str, chosen_pb, queues)

    anchors = {fmt: f"{fmt}-starter-s1" for fmt in FORMATS}
    out = {
        "about": "The ladder's bots (#430 + Gen 2 Playbooks): Career school decks and dedicated ranked decks with "
                 "Forge-inspired AI playbooks, calibrated per format on a shared Glicko-2 scale.",
        "version": 2,
        "anchors": anchors,
        "calibration": previous.get("calibration") if previous else None,
        "bots": bots,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)
        f.write("\n")

    for fmt in FORMATS:
        fb = [b for b in bots if b["format"] == fmt and not b.get("retired")]
        open_b = [b for b in fb if not b.get("queues") or "open" in b["queues"]]
        decks = {b["deck"] for b in open_b}
        gen2 = sum(1 for b in fb if b["generation"] == 2)
        print(f"  {fmt:8s}: {len(fb):3d} bots ({gen2:2d} Gen 2), {len(decks):2d} unique Open decks")
    print(f"Total: {len(bots)} bots -> {os.path.relpath(OUT)}")


if __name__ == "__main__":
    main()

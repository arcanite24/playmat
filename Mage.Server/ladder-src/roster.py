#!/usr/bin/env python3
"""The ladder's bot roster (#430): every Standard and Pioneer school deck at the AI skills its act suits, each bot with
its own persona drawn from the cosmetics catalog.

    python3 Mage.Server/ladder-src/roster.py

Writes Mage.Server/src/main/resources/ladder/bots.json. Run it again after adding a school deck or changing the bands:
bots that keep their id keep their persona and their measured rating; new bots start at their prior, uncalibrated,
until `ladder-calibrate` (rust/crates/playmat-ai/src/bin/ladder-calibrate.rs) measures them.
"""
import hashlib
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.dirname(HERE)
RESOURCES = os.path.join(SERVER, "src", "main", "resources")
DECKS = os.path.join(RESOURCES, "career", "campaigns", "decks")
OUT = os.path.join(RESOURCES, "ladder", "bots.json")
FORMATS = ["standard", "pioneer"]

# What a skill plays on the Rust host (playmat-ai entrants::skill): 1 quick answers, 3 a fifth of the budget,
# 5 half, 7 the full search.
SKILLS = {1: "quick", 3: "light", 5: "half", 7: "full"}
# The skills each kind of school deck is fielded at: the weakest decks never meet the full search, the strongest
# never play without one.
BANDS = {"starter": [1, 3, 5], "signature": [1, 3, 5], "act1": [1, 3, 5], "act2": [3, 5, 7], "act3": [5, 7], "meta": [5, 7]}
# A first guess at the rating before calibration: what the deck and the search add.
PRIOR_DECK = {"starter": 0, "signature": 0, "act1": 50, "act2": 150, "act3": 250, "meta": 300}
PRIOR_SKILL = {1: 0, 3: 150, 5: 250, 7: 350}
# Better bots dress better: the rarities a bot's cosmetics are drawn from, by skill.
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
    for k in ("starter", "signature", "meta", "act1", "act2", "act3"):
        if stem.startswith(k):
            return k
    return None


def deck_name(path):
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("NAME:"):
                return line[5:].strip()
    return os.path.splitext(os.path.basename(path))[0]


def colors(path):
    """the deck's colours from its basic lands, as WUBRG letters (a rough signature for the persona's tagline)"""
    basics = {"Plains": "W", "Island": "U", "Swamp": "B", "Mountain": "R", "Forest": "G"}
    found = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            m = re.match(r"\d+ (?:\[[^\]]+\] )?(.+)$", line.strip())
            if m and m.group(1) in basics:
                found.add(basics[m.group(1)])
    return "".join(c for c in "WUBRG" if c in found)


def h(text):
    return int(hashlib.sha256(text.encode()).hexdigest(), 16)


def main():
    cosmetics = json.load(open(os.path.join(RESOURCES, "career", "cosmetics.json"), encoding="utf-8"))
    pool = {}
    for item in cosmetics:
        if item["kind"] in ("avatar", "sleeve", "playmat", "frame", "title") and not item.get("rank") and not item.get("mastery"):
            pool.setdefault(item["kind"], []).append(item)
    old = {}
    if os.path.exists(OUT):
        previous = json.load(open(OUT, encoding="utf-8"))
        old = {b["id"]: b for b in previous.get("bots", [])}
    used_names = {b["persona"]["name"] for b in old.values()}

    def pick(kind, skill, seed):
        items = [i for i in pool[kind] if i.get("rarity") in RARITIES[skill]] or pool[kind]
        items.sort(key=lambda i: i["id"])
        return items[h(seed + kind) % len(items)]["id"]

    def name_for(seed):
        n = h(seed)
        for k in range(len(GIVEN) * len(FAMILY)):
            name = GIVEN[(n + k) % len(GIVEN)] + " " + FAMILY[(n // len(GIVEN) + k * 7) % len(FAMILY)]
            if name not in used_names:
                used_names.add(name)
                return name
        raise SystemExit("ran out of persona names")

    bots = []
    for fmt in FORMATS:
        folder = os.path.join(DECKS, fmt)
        for file in sorted(os.listdir(folder)):
            stem, ext = os.path.splitext(file)
            kind = kind_of(stem)
            if ext != ".dck" or kind is None:
                continue
            path = os.path.join(folder, file)
            for skill in BANDS[kind]:
                bot_id = f"{fmt}-{stem}-s{skill}"
                prior = 1000 + PRIOR_DECK[kind] + PRIOR_SKILL[skill]
                before = old.get(bot_id)
                persona = before["persona"] if before else {
                    "name": name_for(bot_id),
                    "avatar": pick("avatar", skill, bot_id),
                    "sleeve": pick("sleeve", skill, bot_id),
                    "playmat": pick("playmat", skill, bot_id),
                    "frame": pick("frame", skill, bot_id),
                    "title": pick("title", skill, bot_id),
                }
                bot = {
                    "id": bot_id,
                    "format": fmt,
                    "deck": f"{fmt}/{file}",
                    "deckName": deck_name(path),
                    "colors": colors(path),
                    "kind": kind,
                    "skill": skill,
                    "search": SKILLS[skill],
                    "prior": prior,
                    "persona": persona,
                }
                for key in ("rating", "rd", "games", "won", "drew", "lost"):
                    if before and key in before:
                        bot[key] = before[key]
                bots.append(bot)
    anchors = {fmt: f"{fmt}-starter-s1" for fmt in FORMATS}
    out = {
        "about": "The ladder's bots (#430): a school deck at a Rust host AI skill, with a persona. Generated by "
                 "Mage.Server/ladder-src/roster.py; ratings measured by ladder-calibrate. Each format is its own "
                 "scale, anchored so its starter at skill 1 is 1000.",
        "version": 1,
        "anchors": anchors,
        "calibration": previous.get("calibration") if os.path.exists(OUT) else None,
        "bots": bots,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)
        f.write("\n")
    by = {fmt: sum(1 for b in bots if b["format"] == fmt) for fmt in FORMATS}
    print(f"{len(bots)} bots: " + ", ".join(f"{fmt} {n}" for fmt, n in by.items()) + f" -> {os.path.relpath(OUT)}")


if __name__ == "__main__":
    main()

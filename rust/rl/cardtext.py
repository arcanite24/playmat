"""Card text features (#400 R6): a fixed vector per card-name bucket from the card's rules text and type line, so a
card the network never saw in training starts from what its text says rather than from a random embedding.

    python cardtext.py            # writes data/cardtext.npy (BUCKETS x WIDTH, float16)
    python cardtext.py --holdout 0.1   # also writes data/holdout.json: a tenth of the benchmark's cards, for
                                       # `train.py --holdout` (the cold-start test: their own embeddings stay 0)

Each card's words and word pairs (the card's own name replaced by `{this}`, numbers kept) are hashed into WIDTH
slots, weighted by idf over the card pool, and the row is scaled to unit length. Buckets shared by several of the
engine's verified cards (`playmat-engine/cards/verified.txt`) get their mean. `model.Tx(text=1)` learns a projection
of these rows that is added to the name embedding; `model.export` folds the two into one table, so `net.rs` reads the
same file format.
"""

import json
import math
import os
import random
import re
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
CARDS = os.path.join(HERE, "..", "crates", "playmat-engine", "cards")
BUCKETS = 4096
WIDTH = 1024


def card_id(name):
    """obs.rs `card_id`: FNV-1a 64 of the name's bytes, into buckets 1.."""
    h = 0xCBF29CE484222325
    for b in name.encode():
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return 1 + h % (BUCKETS - 1)


def fnv(s):
    h = 0xCBF29CE484222325
    for b in s.encode():
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def words(name, view):
    text = " ".join(view.get("rules") or []).replace(name, "{this}")
    types = " ".join(t.lower() for t in view.get("cardTypes") or [])
    subs = " ".join(s.lower() for s in view.get("subTypes") or [])
    toks = re.findall(r"\{[^}]*\}|[a-z0-9+/\-]+", (text + " " + types + " " + subs).lower())
    return toks + [a + " " + b for a, b in zip(toks, toks[1:])]


def row_of(toks, df, n):
    """A card's hashed, idf-weighted, unit-length bag of words"""
    row = np.zeros(WIDTH, np.float32)
    for t in toks:
        h = fnv(t)
        row[h % WIDTH] += (1.0 if (h >> 32) & 1 else -1.0) * math.log(n / df[t])
    norm = np.linalg.norm(row)
    return row / norm if norm > 0 else row


TACTICAL_TAGS = [
    r"\bdestroy\b",
    r"\bexile\b",
    r"\bdeals? \d+ damage|\bdeals? x damage|\bdamage to any target",
    r"\bcounter target\b",
    r"to its owner's hand|to their owners' hands",
    r"\bdiscard",
    r"\bdraw a card|\bdraw \w+ cards",
    r"add \{|search your library for a .*land",
    r"all creatures|each creature",
    r"gets \+\d+/\+\d+|get \+\d+/\+\d+",
    r"prevent all combat damage|prevent the next",
    r"\bgain \d+ life|\blifelink\b",
    r"create a .* token|create .* tokens",
    r"from your graveyard|from a graveyard|\bescape\b|\bflashback\b|\bunearth\b",
    r"\bsacrifice",
    r"\bhexproof\b|\bindestructible\b|\bprotection from\b",
    r"\btap target|\bstun counter",
    r"\bfights?\b|deals damage equal to its power",
    r"\bscry\b|\bsurveil\b",
    r"\bflash\b",
    r"target artifact|target enchantment",
    r"choose one|choose two",
    r"becomes a \d+/\d+ .* creature",
    r"target player loses \d+ life|each opponent loses",
]


def tag_vec(text):
    t = text.lower()
    return np.array([1.0 if re.search(pat, t) else 0.0 for pat in TACTICAL_TAGS], dtype=np.float32)


def semantic_table(names, views, played, model_name="BAAI/bge-small-en-v1.5"):
    """Embeds each verified card's rules + type line with a pretrained sentence encoder + tactical MTG tags."""
    import torch
    from transformers import AutoModel, AutoTokenizer

    dev = torch.device("mps" if torch.backends.mps.is_available() else ("cuda" if torch.cuda.is_available() else "cpu"))
    tok = AutoTokenizer.from_pretrained(model_name)
    enc = AutoModel.from_pretrained(model_name).to(dev).eval()
    texts = []
    tags = []
    for n in names:
        v = views[n]
        rules = " ".join(v.get("rules") or []).replace(n, "this card")
        types = " ".join(v.get("cardTypes") or [])
        subs = " ".join(v.get("subTypes") or [])
        s = f"{types} {subs}. {rules}".strip()
        texts.append(s)
        tags.append(tag_vec(s))

    embs = []
    bs = 128
    with torch.inference_mode():
        for i in range(0, len(texts), bs):
            inp = tok(texts[i : i + bs], padding=True, truncation=True, max_length=192, return_tensors="pt").to(dev)
            out = enc(**inp).last_hidden_state[:, 0]  # CLS pooling for BGE
            out = torch.nn.functional.normalize(out, p=2, dim=-1)
            embs.append(out.float().cpu().numpy())
    embs = np.concatenate(embs, axis=0)
    tags = np.stack(tags, axis=0) * 0.25  # scale relative to unit-norm embedding
    vecs = np.concatenate([embs, tags], axis=1)
    dim = vecs.shape[1]

    table = np.zeros((BUCKETS, dim), np.float32)
    count = np.zeros(BUCKETS, np.int32)
    by_name = {}
    for n, vec in zip(names, vecs):
        by_name[n] = vec
        cid = card_id(n)
        table[cid] += vec
        count[cid] += 1
    table /= np.maximum(count, 1)[:, None]
    for n in played:
        if n in by_name:
            table[card_id(n)] = by_name[n]
    return table, count


def main():
    import gzip
    views_gz = os.path.join(CARDS, "views.json.gz")
    if os.path.exists(views_gz):
        with gzip.open(views_gz, "rt", encoding="utf-8") as f:
            views = json.load(f)
    else:
        views = json.load(open(os.path.join(CARDS, "views.json")))
    names = [l.strip() for l in open(os.path.join(CARDS, "verified.txt")) if l.strip() and not l.startswith("#")]
    names = [n for n in names if n in views]
    played = set()
    suite = json.load(open(os.path.join(HERE, "..", "..", "archbench", "constructed-2", "suite.json")))
    for deck in suite["decks"]:
        path = os.path.join(HERE, "..", "..", deck["file"])
        for line in open(path) if os.path.exists(path) else []:
            m = re.match(r"\s*(?:SB:\s*)?\d+\s+(.+?)\s*$", line)
            if m and m.group(1) in views:
                played.add(m.group(1))

    if "--semantic" in sys.argv:
        table, count = semantic_table(names, views, played)
        os.makedirs(os.path.join(HERE, "data"), exist_ok=True)
        out_path = os.path.join(HERE, "data", "cardtext_sem.npy")
        np.save(out_path, table.astype(np.float16))
        print(f"CARDTEXT_SEM {len(names)} cards -> {out_path} {table.shape}; {len(played)} benchmark cards keep their own")
        return

    bags = {n: words(n, views[n]) for n in names}
    df = {}
    for toks in bags.values():
        for t in set(toks):
            df[t] = df.get(t, 0) + 1
    table = np.zeros((BUCKETS, WIDTH), np.float32)
    count = np.zeros(BUCKETS, np.int32)
    for n, toks in bags.items():
        row = row_of(toks, df, len(bags))
        if row.any():
            table[card_id(n)] += row
            count[card_id(n)] += 1
    table /= np.maximum(count, 1)[:, None]
    for n in played:
        if n in bags:
            table[card_id(n)] = row_of(bags[n], df, len(bags))
    if "--holdout" in sys.argv:
        frac = float(sys.argv[sys.argv.index("--holdout") + 1])
        basics = {"Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes"}
        pool = sorted(n for n in played if n not in basics)
        pick = sorted(random.Random(400).sample(pool, max(1, round(frac * len(pool)))))
        json.dump({"cards": pick, "ids": sorted({card_id(n) for n in pick})},
                  open(os.path.join(HERE, "data", "holdout.json"), "w"), indent=1)
        print(f"HOLDOUT {len(pick)} of {len(pool)} benchmark cards")
    os.makedirs(os.path.join(HERE, "data"), exist_ok=True)
    np.save(os.path.join(HERE, "data", "cardtext.npy"), table.astype(np.float16))
    print(f"CARDTEXT {len(bags)} cards into {int((count > 0).sum())} buckets ({int((count > 1).sum())} shared); "
          f"{len(played)} benchmark cards keep their own")


if __name__ == "__main__":
    main()

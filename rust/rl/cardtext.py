"""Card text features (#400 R6): a fixed vector per card-name bucket from the card's rules text and type line, so a
card the network never saw in training starts from what its text says rather than from a random embedding.

    python cardtext.py            # writes data/cardtext.npy (BUCKETS x WIDTH, float16)

Each card's words and word pairs (the card's own name replaced by `{this}`, numbers kept) are hashed into WIDTH
slots, weighted by idf over the card pool, and the row is scaled to unit length. Buckets shared by several of the
engine's verified cards (`playmat-engine/cards/verified.txt`) get their mean. `model.Tx(text=1)` learns a projection
of these rows that is added to the name embedding; `model.export` folds the two into one table, so `net.rs` reads the
same file format.
"""

import json
import math
import os
import re

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


def main():
    views = json.load(open(os.path.join(CARDS, "views.json")))
    names = [l.strip() for l in open(os.path.join(CARDS, "verified.txt")) if l.strip() and not l.startswith("#")]
    names = [n for n in names if n in views]
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
    # a bucket that holds a card of the benchmark's decks is that card's alone
    played = set()
    suite = json.load(open(os.path.join(HERE, "..", "..", "archbench", "constructed-2", "suite.json")))
    for deck in suite["decks"]:
        path = os.path.join(HERE, "..", "..", deck["file"])
        for line in open(path) if os.path.exists(path) else []:
            m = re.match(r"\s*(?:SB:\s*)?\d+\s+(.+?)\s*$", line)
            if m and m.group(1) in bags:
                played.add(m.group(1))
    for n in played:
        table[card_id(n)] = row_of(bags[n], df, len(bags))
    os.makedirs(os.path.join(HERE, "data"), exist_ok=True)
    np.save(os.path.join(HERE, "data", "cardtext.npy"), table.astype(np.float16))
    print(f"CARDTEXT {len(bags)} cards into {int((count > 0).sum())} buckets ({int((count > 1).sum())} shared); "
          f"{len(played)} benchmark cards keep their own")


if __name__ == "__main__":
    main()

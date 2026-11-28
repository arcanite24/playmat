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
DEFAULT_REGISTRY_32K = os.path.join(HERE, "..", "crates", "playmat-ai", "src", "card_registry_32k.txt")
BUCKETS = 4096
BUCKETS_32K = 32768
FALLBACK_START_32K = 24577
FALLBACK_SPAN_32K = BUCKETS_32K - FALLBACK_START_32K  # 8191 buckets: 24577..=32767
WIDTH = 1024
CARDTEXT_V2_BGE_DIM = 1024
CARD_SCALAR_DIM = 64
CARDTEXT_V2_DIM = CARDTEXT_V2_BGE_DIM + CARD_SCALAR_DIM  # 1088

_REG_32K_CACHE = {}


def load_registry_32k(path=None):
    """Loads frozen `card_registry_32k.txt` -> `(names_list, {name: 1_based_id})` (`1..=16483`)."""
    cand_paths = [path] if path else [
        DEFAULT_REGISTRY_32K,
        os.path.join(HERE, "data", "card_registry_32k.txt"),
        os.path.join(HERE, "..", "..", "cardtext", "card_registry_32k.txt"),
    ]
    resolved = None
    for p in cand_paths:
        if p and os.path.exists(p):
            resolved = os.path.abspath(p)
            break
    if resolved is None:
        raise FileNotFoundError(f"card_registry_32k.txt not found in {cand_paths}")
    if resolved not in _REG_32K_CACHE:
        with open(resolved, encoding="utf-8") as f:
            names = [ln.rstrip("\r\n") for ln in f if ln.strip()]
        mapping = {n: i + 1 for i, n in enumerate(names)}
        assert len(mapping) == len(names), "duplicate card names in card_registry_32k.txt"
        assert len(names) < FALLBACK_START_32K, f"registry size {len(names)} exceeds fallback start {FALLBACK_START_32K}"
        _REG_32K_CACHE[resolved] = (names, mapping)
    return _REG_32K_CACHE[resolved]


def fnv1a_64(data_bytes):
    h = 0xCBF29CE484222325
    for b in data_bytes:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def card_id(name):
    """obs.rs `card_id`: FNV-1a 64 of the name's bytes, into buckets 1..4095"""
    if not name:
        return 0
    return 1 + fnv1a_64(name.encode("utf-8")) % (BUCKETS - 1)


def card_id_32k(name, registry_path=None):
    """Frozen 32,768-slot canonical card ID (`obs::card_id_for_buckets(name, 32768)`):
    - `0` for empty/none
    - `1..=16483` exact 1-based line index in `card_registry_32k.txt` (0.0% collisions)
    - `24577..=32767` FNV-1a 64 fallback band for unverified/token/synthetic names
    """
    if not name:
        return 0
    _, reg_map = load_registry_32k(registry_path)
    cid = reg_map.get(name)
    if cid is not None:
        return cid
    return FALLBACK_START_32K + (fnv1a_64(name.encode("utf-8")) % FALLBACK_SPAN_32K)


def card_id_for_buckets(name, buckets=BUCKETS, registry_path=None):
    if not name:
        return 0
    b = int(buckets)
    if b == BUCKETS_32K:
        return card_id_32k(name, registry_path=registry_path)
    return 1 + (fnv1a_64(name.encode("utf-8")) % (b - 1))


def fnv(s):
    return fnv1a_64(s.encode("utf-8"))


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


_HTML_TAG_RE = re.compile(r"<[^>]+>")
_MANA_SYM_RE = re.compile(r"\{([^}]+)\}")
_NUM_PREFIX_RE = re.compile(r"^[+-]?\d+")

TACTICAL_TAGS_V2 = [
    re.compile(r"\bdestroy\b"),
    re.compile(r"\bexile\b"),
    re.compile(r"\bdeals? (?:\d+|x) damage|\bdamage to any target"),
    re.compile(r"\bcounter target\b"),
    re.compile(r"to its owner's hand|to their owners' hands"),
    re.compile(r"\bdiscard"),
    re.compile(r"\bdraw (?:a|\w+) cards?"),
    re.compile(r"add \{|search your library for a .*land"),
    re.compile(r"all creatures|each creature|each nonland permanent"),
    re.compile(r"gets? [+-]\d+/[+-]\d+|\+1/\+1 counter"),
    re.compile(r"prevent all combat damage|prevent the next"),
    re.compile(r"\bgain (?:\d+|x) life|\blifelink\b"),
    re.compile(r"create (?:a|\w+) .*tokens?"),
    re.compile(r"from (?:your|a) graveyard|\bescape\b|\bflashback\b|\bunearth\b|\bdelve\b|\bdisturb\b"),
    re.compile(r"\bsacrifice\b"),
    re.compile(r"\bhexproof\b|\bward\b|\bindestructible\b|\bprotection from\b"),
    re.compile(r"\btap target|\bstun counter"),
    re.compile(r"\bfights?\b|deals damage equal to its power"),
    re.compile(r"\bscry\b|\bsurveil\b|search your library"),
    re.compile(r"\bflying\b|\breach\b"),
    re.compile(r"\btrample\b|\bmenace\b|can't be blocked"),
    re.compile(r"\bhaste\b|\bvigilance\b"),
    re.compile(r"\bfirst strike\b|\bdouble strike\b|\bdeathtouch\b"),
    re.compile(r"choose one|choose two|^[+-]\d+:|0:|i - |ii - |iii - "),
]
assert len(TACTICAL_TAGS_V2) == 24


def _clean_rules(lines, card_name):
    if not lines:
        return ""
    raw = " ".join(str(x) for x in lines if x)
    if card_name:
        raw = raw.replace(card_name, "{this}")
    raw = _HTML_TAG_RE.sub(" ", raw)
    return " ".join(raw.split())


def _parse_stat_num(val):
    s = str(val or "").strip()
    if not s:
        return 0.0, False
    has_var = ("*" in s) or ("x" in s.lower())
    m = _NUM_PREFIX_RE.match(s)
    num = float(int(m.group(0))) if m else 0.0
    return num, has_var


def _parse_mana_symbols(cost_syms):
    """Returns `(wubrgc[6], generic, has_x, has_hybrid_or_phyrexian)` from a list of `{...}` strings."""
    pips = np.zeros(6, dtype=np.float32)
    generic = 0.0
    has_x = 0.0
    has_hyb_phy = 0.0
    col_idx = {"W": 0, "U": 1, "B": 2, "R": 3, "G": 4, "C": 5}
    raw = "".join(cost_syms or [])
    for tok in _MANA_SYM_RE.findall(raw.upper()):
        parts = tok.split("/")
        if len(parts) == 1:
            p = parts[0]
            if p.isdigit():
                generic += float(int(p))
            elif p in ("X", "Y", "Z"):
                has_x = 1.0
            elif p in col_idx:
                pips[col_idx[p]] += 1.0
        else:
            has_hyb_phy = 1.0
            cols = [p for p in parts if p in col_idx]
            if "P" in parts and len(cols) == 1:
                pips[col_idx[cols[0]]] += 1.0
            elif len(cols) >= 2:
                w = 1.0 / len(cols)
                for c in cols:
                    pips[col_idx[c]] += w
            elif len(cols) == 1:
                pips[col_idx[cols[0]]] += 0.5
                for p in parts:
                    if p.isdigit():
                        generic += 0.5 * float(int(p))
    return pips, generic, has_x, has_hyb_phy


def format_card_sheet(name, view):
    """Formats the complete oracle card sheet for BGE-large (`cardtext_v2` frozen template)."""
    v = view if isinstance(view, dict) else {}
    supertypes = " ".join(s.title() for s in (v.get("superTypes") or []))
    types = " ".join(t.title() for t in (v.get("cardTypes") or []))
    subtypes = " ".join(s.title() for s in (v.get("subTypes") or []))
    cost_syms = v.get("manaCostLeftStr") or []
    mana_cost = "".join(cost_syms) if cost_syms else "None"
    mv = int(v.get("manaValue") or 0)
    cdict = v.get("color") or {}
    col_letters = [
        letter
        for key, letter in (("white", "W"), ("blue", "U"), ("black", "B"), ("red", "R"), ("green", "G"))
        if cdict.get(key)
    ]
    colors = "".join(col_letters) if col_letters else "Colorless"
    ctype_set = {t.upper() for t in (v.get("cardTypes") or [])}
    if "CREATURE" in ctype_set or "VEHICLE" in {s.upper() for s in (v.get("subTypes") or [])}:
        p = str(v.get("power") if v.get("power") not in (None, "") else "0")
        t = str(v.get("toughness") if v.get("toughness") not in (None, "") else "0")
    else:
        p, t = "-", "-"
    loy = str(v.get("startingLoyalty") or v.get("loyalty") or v.get("startingDefense") or v.get("defense") or "-")
    rules = _clean_rules(v.get("rules") or [], name)
    r_name = v.get("rightSplitName") or ""
    r_rules = _clean_rules(v.get("rightSplitRules") or [], r_name or name)
    if r_name or r_rules:
        r_type = (v.get("rightSplitTypeLine") or "").strip()
        r_cost = v.get("rightSplitCostsStr") or "".join(v.get("manaCostRightStr") or [])
        rules = f"{rules} // {r_name} ({r_type}, Cost: {r_cost}): {r_rules}".strip()
    bf = v.get("secondCardFace")
    if isinstance(bf, dict):
        bf_name = bf.get("displayName") or bf.get("name") or "Back"
        bf_types = " ".join(x.title() for x in ((bf.get("superTypes") or []) + (bf.get("cardTypes") or []) + (bf.get("subTypes") or [])))
        bf_rules = _clean_rules(bf.get("rules") or [], bf_name)
        bf_p = bf.get("power") or "-"
        bf_t = bf.get("toughness") or "-"
        rules = f"{rules} // Back {bf_name} ({bf_types}, P/T: {bf_p}/{bf_t}): {bf_rules}".strip()
    lead = f"{supertypes} {types}".strip()
    return f"{lead} - {subtypes} | Cost: {mana_cost} (MV {mv}, Colors: {colors}) | P/T: {p}/{t} | Loyalty: {loy} | {rules}".strip()


def sheet_scalars_64(name, view):
    """Extracts 64 deterministic, bounded structured card-sheet scalars (0 NaNs, scaled by 0.25 at concat)."""
    v = view if isinstance(view, dict) else {}
    out = np.zeros(CARD_SCALAR_DIM, dtype=np.float32)
    cost_syms = list(v.get("manaCostLeftStr") or [])
    pips, generic, has_x, has_hyb_phy = _parse_mana_symbols(cost_syms)
    out[0:6] = np.clip(pips / 3.0, 0.0, 2.0)
    out[6] = min(generic / 6.0, 2.5)
    out[7] = has_x
    out[8] = has_hyb_phy
    out[9] = min(max(float(v.get("manaValue") or 0), 0.0) / 6.0, 2.5)

    cdict = v.get("color") or {}
    cols = [
        1.0 if cdict.get(k) else 0.0
        for k in ("white", "blue", "black", "red", "green")
    ]
    out[10:15] = cols
    n_cols = sum(cols)
    out[15] = 1.0 if n_cols >= 2.0 else 0.0
    out[16] = 1.0 if n_cols == 0.0 else 0.0

    cid_str = (v.get("originalColorIdentity") or "").upper()
    for i, (sym, c_val) in enumerate(zip(("W", "U", "B", "R", "G"), cols)):
        out[17 + i] = 1.0 if (c_val > 0.0 or sym in cid_str) else 0.0

    ctypes = {t.upper() for t in (v.get("cardTypes") or [])}
    for i, ct in enumerate(("ARTIFACT", "CREATURE", "ENCHANTMENT", "INSTANT", "LAND", "PLANESWALKER", "SORCERY", "BATTLE")):
        out[22 + i] = 1.0 if ct in ctypes else 0.0

    stypes = {s.upper() for s in (v.get("superTypes") or [])}
    out[30] = 1.0 if "LEGENDARY" in stypes else 0.0
    out[31] = 1.0 if "BASIC" in stypes else 0.0
    out[32] = 1.0 if "SNOW" in stypes else 0.0
    out[33] = 1.0 if (v.get("isSplitCard") or v.get("isDoubleFacedCard")) else 0.0

    is_creat = ("CREATURE" in ctypes) or ("VEHICLE" in {s.upper() for s in (v.get("subTypes") or [])})
    p_num, p_var = _parse_stat_num(v.get("power")) if is_creat else (0.0, False)
    t_num, t_var = _parse_stat_num(v.get("toughness")) if is_creat else (0.0, False)
    loy_raw = v.get("startingLoyalty") or v.get("loyalty") or v.get("startingDefense") or v.get("defense") or ""
    loy_num, _ = _parse_stat_num(loy_raw)
    full_rules = (
        _clean_rules(v.get("rules") or [], name)
        + " "
        + _clean_rules(v.get("rightSplitRules") or [], name)
    ).lower()
    if isinstance(v.get("secondCardFace"), dict):
        full_rules += " " + _clean_rules(v["secondCardFace"].get("rules") or [], name).lower()

    out[34] = float(np.clip(p_num / 5.0, -0.5, 3.0))
    out[35] = float(np.clip(t_num / 5.0, -0.5, 3.0))
    out[36] = 1.0 if (p_var or t_var) else 0.0
    out[37] = float(np.clip(loy_num / 5.0, 0.0, 2.5))
    out[38] = 1.0 if ("INSTANT" in ctypes or re.search(r"\bflash\b", full_rules)) else 0.0
    out[39] = 1.0 if (bool(v.get("rightSplitRules")) or bool(v.get("isDoubleFacedCard"))) else 0.0

    for i, pat in enumerate(TACTICAL_TAGS_V2):
        out[40 + i] = 1.0 if pat.search(full_rules) else 0.0
    return out


def build_v2_tables(
    views,
    registry_path=None,
    model_name="BAAI/bge-large-en-v1.5",
    batch_size=256,
    max_length=512,
    device=None,
    encode_fn=None,
):
    """Builds `(table_32k [32768, 1088], table_4k [4096, 1088])` in float32 using BGE-large (1024d) + 64d scalars.
    - `table_32k`: exact 1:1 slots `1..=16483` from `card_registry_32k.txt` (0% collisions, 0% C2 overwrites),
      plus fallback FNV-1a slots `24577..=32767` for any remaining non-registry dict views in `views`.
    - `table_4k`: bucket-averaged across `card_registry_32k.txt` into `1..4095` with L2-renormalized 1024d BGE
      vectors and NO `constructed-2` overwrite bug.
    """
    reg_names, reg_map = load_registry_32k(registry_path)
    extra_names = sorted(
        k for k, v in views.items()
        if isinstance(v, dict) and not k.startswith("abilities:") and k not in reg_map
    )
    all_names = list(reg_names) + extra_names
    sheets = [format_card_sheet(n, views.get(n)) for n in all_names]
    scalars = np.stack([sheet_scalars_64(n, views.get(n)) for n in all_names], axis=0) * 0.25

    if encode_fn is not None:
        bge = np.asarray(encode_fn(sheets), dtype=np.float32)
    else:
        import torch
        from transformers import AutoModel, AutoTokenizer

        if device is None:
            dev = torch.device("cuda" if torch.cuda.is_available() else ("mps" if torch.backends.mps.is_available() else "cpu"))
        else:
            dev = torch.device(device)
        tok = AutoTokenizer.from_pretrained(model_name)
        enc = AutoModel.from_pretrained(model_name).to(dev).eval()
        amp_cuda = dev.type == "cuda"
        amp_dtype = torch.bfloat16 if (amp_cuda and torch.cuda.is_bf16_supported()) else torch.float16

        embs = []
        with torch.inference_mode():
            for i in range(0, len(sheets), batch_size):
                inp = tok(
                    sheets[i : i + batch_size],
                    padding=True,
                    truncation=True,
                    max_length=max_length,
                    return_tensors="pt",
                ).to(dev)
                with torch.autocast("cuda", dtype=amp_dtype, enabled=amp_cuda):
                    out = enc(**inp).last_hidden_state[:, 0]  # CLS pooling for BGE
                out = torch.nn.functional.normalize(out.float(), p=2, dim=-1)
                embs.append(out.cpu().numpy())
        bge = np.concatenate(embs, axis=0).astype(np.float32)
    assert bge.shape == (len(all_names), CARDTEXT_V2_BGE_DIM), f"expected 1024d BGE, got {bge.shape}"
    vecs = np.concatenate([bge, scalars], axis=1).astype(np.float32)
    assert vecs.shape[1] == CARDTEXT_V2_DIM and np.isfinite(vecs).all()

    n_reg = len(reg_names)
    # 1. Collision-free 32,768 table
    table_32k = np.zeros((BUCKETS_32K, CARDTEXT_V2_DIM), dtype=np.float32)
    for idx in range(n_reg):
        table_32k[idx + 1] = vecs[idx]
    if extra_names:
        fb_cnt = np.zeros(BUCKETS_32K, dtype=np.int32)
        for j, nm in enumerate(extra_names):
            cid = FALLBACK_START_32K + (fnv1a_64(nm.encode("utf-8")) % FALLBACK_SPAN_32K)
            table_32k[cid] += vecs[n_reg + j]
            fb_cnt[cid] += 1
        fb_used = fb_cnt > 0
        table_32k[fb_used] /= fb_cnt[fb_used, None]
        fb_norms = np.linalg.norm(table_32k[fb_used, :CARDTEXT_V2_BGE_DIM], axis=1, keepdims=True)
        table_32k[fb_used, :CARDTEXT_V2_BGE_DIM] /= np.maximum(fb_norms, 1e-8)

    # 2. Clean 4,096 table (bucket-averaged over verified registry cards, L2-renormalized BGE, NO C2 overwrite!)
    table_4k = np.zeros((BUCKETS, CARDTEXT_V2_DIM), dtype=np.float32)
    cnt_4k = np.zeros(BUCKETS, dtype=np.int32)
    for idx, nm in enumerate(reg_names):
        cid4 = card_id(nm)
        table_4k[cid4] += vecs[idx]
        cnt_4k[cid4] += 1
    used_4k = cnt_4k > 0
    table_4k[used_4k] /= cnt_4k[used_4k, None]
    norms_4k = np.linalg.norm(table_4k[used_4k, :CARDTEXT_V2_BGE_DIM], axis=1, keepdims=True)
    table_4k[used_4k, :CARDTEXT_V2_BGE_DIM] /= np.maximum(norms_4k, 1e-8)
    return table_32k, table_4k


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

    if "--v2" in sys.argv:
        model_name = "BAAI/bge-large-en-v1.5"
        if "--model" in sys.argv:
            model_name = sys.argv[sys.argv.index("--model") + 1]
        out_dir = os.path.join(HERE, "data")
        if "--out-dir" in sys.argv:
            out_dir = sys.argv[sys.argv.index("--out-dir") + 1]
        reg_path = None
        if "--registry" in sys.argv:
            reg_path = sys.argv[sys.argv.index("--registry") + 1]
        os.makedirs(out_dir, exist_ok=True)
        t32, t4 = build_v2_tables(views, registry_path=reg_path, model_name=model_name)
        p32 = os.path.join(out_dir, "cardtext_v2_32k.npy")
        p4 = os.path.join(out_dir, "cardtext_v2_4k.npy")
        np.save(p32, t32.astype(np.float16))
        np.save(p4, t4.astype(np.float16))
        print(f"CARDTEXT_V2 wrote {p32} {t32.shape} and {p4} {t4.shape} (model={model_name})", flush=True)
        return

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

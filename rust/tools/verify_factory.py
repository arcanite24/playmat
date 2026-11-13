#!/usr/bin/env python3
"""The verification factory (docs/rust/Engine.md, "Pool, gate and factory"): checks cards that are in the Rust engine
but not verified against Java, at scale, with shadow games.

    python3 rust/tools/verify_factory.py build    --work W [--limit N] [--candidates FILE] [--priority TSV]
    python3 rust/tools/verify_factory.py record   --work W [--round R] [--procs 8]
    python3 rust/tools/verify_factory.py replay   --work W [--round R | --all]
    python3 rust/tools/verify_factory.py verdicts --work W [--min-games 3] [--write]
    python3 rust/tools/verify_factory.py run      --work W [--limit N] [--procs 8] [--write]   # all four
    python3 rust/tools/verify_factory.py views    --work W    # cards/views.json for the verified cards

`build` takes the candidates (by default every card in cards.json that isn't on cards/verified.txt, in the order of
a priority list such as card-priority.tsv's score when one is given, else by name) and makes a round of legal
60-card decks around them: each deck has two colors, about 6 candidate spells and 2 candidate lands (4 copies each),
3 verified spells of its colors, and basics; each candidate goes into `--decks-per-card` decks (3) with different
partners. It pairs each deck against `--games-per-deck` (2) others and writes the seeds that make ShadowGamesTest
play exactly those pairs (`a = N % n`, `b = (a + 1 + (N / n) % (n - 1)) % n`).

`record` runs ShadowGamesTest over the round's seeds in `--procs` parallel Maven processes (the card database is an
H2 file opened in auto-server mode, so they can share it). Long: run it in the background. The Java recordings are
kept, and `replay --all` replays every round again after an engine fix.

`replay` runs the playmat-engine shadow test on a round's recordings (`PLAYMAT_SHADOW`) and keeps, per game, whether
it matched and, if not, Java's and Rust's texts where it parted.

`verdicts` reads every round: a card becomes verified after `--min-games` (3) matching games that exercised it (it
was on the stack: cast, activated or triggered; a land, on the battlefield) with no mismatching game blamed on it. A
mismatching game is blamed on the first card involved: the unverified card on the stack where Java and Rust part,
else one named on a line that differs, else the unverified card that was last on the stack before it. It writes
`verdicts.tsv` and `bugs.md` (mismatches clustered by the blamed card) in the work folder; `--write` adds the
newly verified cards to cards/verified.txt.

`views` writes cards/views.json again for the verified cards (CardFactsTest with views, then view_templates.py):
a verified card needs a view template, the rest of the pool doesn't (docs/rust/Views.md).
"""
import argparse
import collections
import glob
import json
import os
import random
import re
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
ENGINE = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine')
CARDS = os.path.join(ENGINE, 'cards', 'cards.json')
FACTS = os.path.join(ENGINE, 'cards', 'facts.json')
VERIFIED = os.path.join(ENGINE, 'cards', 'verified.txt')
COLORS = 'WUBRG'
BASIC = {'W': 'Plains', 'U': 'Island', 'B': 'Swamp', 'R': 'Mountain', 'G': 'Forest'}
PAIRS = [a + b for i, a in enumerate(COLORS) for b in COLORS[i + 1:]]
LAND_MANA = {'WhiteManaAbility': 'W', 'BlueManaAbility': 'U', 'BlackManaAbility': 'B', 'RedManaAbility': 'R',
             'GreenManaAbility': 'G'}


def load_cards():
    return {c['name']: c for c in json.load(open(CARDS))}


def load_verified():
    if not os.path.exists(VERIFIED):
        return set()
    return {l.strip() for l in open(VERIFIED, encoding='utf-8') if l.strip() and not l.startswith('#')}


def colors_of(card):
    """a card's colors: its cost's symbols (hybrid and Phyrexian count each color), else its color indicator"""
    out = set(card.get('colors') or [])
    for sym in re.findall(r'\{([^}]*)\}', card.get('cost') or ''):
        out.update(ch for ch in sym if ch in COLORS)
    for face in ('back', 'transform_back', 'adventure'):
        f = card.get(face)
        if isinstance(f, dict) and face == 'adventure':
            for sym in re.findall(r'\{([^}]*)\}', f.get('cost') or ''):
                out.update(ch for ch in sym if ch in COLORS)
    return ''.join(c for c in COLORS if c in out)


def land_colors(card):
    """the colors a land's own mana abilities make (none for a utility land)"""
    out = set()

    def walk(x):
        if isinstance(x, dict):
            c = x.get('class')
            if c in LAND_MANA:
                out.add(LAND_MANA[c])
            for v in x.values():
                walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(card.get('abilities', []))
    return ''.join(c for c in COLORS if c in out)


def is_land(card):
    return 'LAND' in card.get('types', [])


def mana_value(card):
    n = 0
    for sym in re.findall(r'\{([^}]*)\}', card.get('cost') or ''):
        n += int(sym) if sym.isdigit() else 0 if sym in ('X', 'Y') else 1
    return n


def load_errors():
    """the cards of cards.json the engine can't load (lib.rs's test prints them), which can't be played"""
    p = subprocess.run(['cargo', '+stable', 'test', '--release', '-q', '-p', 'playmat-engine', '--lib',
                        'every_verified_card_loads', '--', '--nocapture'], cwd=os.path.join(ROOT, 'rust'),
                       capture_output=True, text=True)
    out, on = set(), False
    for line in p.stderr.split('\n'):
        if "can't load yet" in line:
            on = True
            continue
        if on:
            if ': ' not in line:
                break
            out.add(line.split(': ')[0])
    return out


def priority_order(path):
    """card name -> score, from a TSV with `card` and `score` columns (the port data's card-priority.tsv)"""
    if not path or not os.path.exists(path):
        return {}
    rows = [l.rstrip('\n').split('\t') for l in open(path, encoding='utf-8')]
    head = rows[0]
    ci, si = head.index('card'), head.index('score')
    return {r[ci]: int(r[si]) for r in rows[1:] if len(r) > si and r[si].lstrip('-').isdigit()}


# ---------------------------------------------------------------------------------------------------------- rounds

def rounds(work):
    return sorted(int(re.sub(r'\D', '', os.path.basename(d))) for d in glob.glob(os.path.join(work, 'round-*')))


def round_dir(work, r):
    return os.path.join(work, 'round-%03d' % r)


def settled(work):
    """cards a later round shouldn't take again: verified, or blamed for a mismatch, or already tried in
    `--max-tries` rounds without enough games (see `verdicts`)"""
    path = os.path.join(work, 'verdicts.tsv')
    if not os.path.exists(path):
        return {}
    out = {}
    for line in open(path, encoding='utf-8').read().splitlines()[1:]:
        f = line.split('\t')
        out[f[0]] = {'verdict': f[1], 'rounds': int(f[5])}
    return out


def build(args):
    cards = load_cards()
    verified = load_verified() & set(cards)
    scores = priority_order(args.priority)
    if args.candidates:
        names = [l.strip() for l in open(args.candidates, encoding='utf-8') if l.strip()]
    else:
        names = sorted(set(cards) - verified, key=lambda n: (-scores.get(n, 0), n))
        if args.used_only:
            names = [n for n in names if scores.get(n, 0) > 0]
    broken = load_errors()
    names = [n for n in names if n not in broken]
    prior = settled(args.work)
    names = [n for n in names if n in cards and n not in verified and not n.startswith('A-')
             and prior.get(n, {}).get('verdict') not in ('verified', 'bug')
             and prior.get(n, {}).get('rounds', 0) < args.max_tries
             and 'BASIC' not in cards[n].get('supertypes', [])]
    if args.limit:
        names = names[:args.limit]
    if not names:
        print('no candidates left', file=sys.stderr)
        return None
    r = (rounds(args.work) or [0])[-1] + 1
    out = round_dir(args.work, r)
    decks_dir = os.path.join(out, 'decks')
    os.makedirs(decks_dir, exist_ok=True)
    rng = random.Random(args.seed + r)

    # verified fillers by color: cheap nonland spells, creatures first
    fillers = collections.defaultdict(list)
    for n in sorted(verified):
        c = cards[n]
        if is_land(c) or 'BASIC' in c.get('supertypes', []) or not c.get('cost'):
            continue
        if 'LEGENDARY' in c.get('supertypes', []) or mana_value(c) > 3:
            continue
        fillers[colors_of(c)].append(n)

    # each candidate's placements: `decks_per_card` buckets of deck colors that hold it
    spells, lands = collections.defaultdict(list), collections.defaultdict(list)
    for i, n in enumerate(names):
        c = cards[n]
        if is_land(c):
            col = land_colors(c)
            fits = [p for p in PAIRS if set(col) & set(p)] if col else PAIRS
            target = lands
        else:
            col = colors_of(c)
            if len(col) > 2:
                fits = [col]
            elif len(col) == 2:
                fits = [col]
            else:
                fits = [p for p in PAIRS if set(col) <= set(p)]
            target = spells
        start = rng.randrange(len(fits))
        for k in range(args.decks_per_card):
            target[fits[(start + k) % len(fits)]].append((i, n))

    decks = []
    for colors in sorted(set(spells) | set(lands)):
        s_list, l_list = spells.get(colors, []), lands.get(colors, [])
        # one deck per `per_deck` spells (and its share of the lands), spreading a card's copies over decks
        n_decks = max(1, -(-len(s_list) // args.per_deck), -(-len(l_list) // args.lands_per_deck))
        groups_s = [[] for _ in range(n_decks)]
        groups_l = [[] for _ in range(n_decks)]
        for lst, groups in ((s_list, groups_s), (l_list, groups_l)):
            # the same card goes to different decks: deal round-robin in priority order
            for j, (_, n) in enumerate(sorted(lst)):
                g = j % n_decks
                tries = 0
                while n in groups[g] and tries < n_decks:
                    g = (g + 1) % n_decks
                    tries += 1
                groups[g].append(n)
        for k in range(n_decks):
            decks.append((colors, groups_s[k], groups_l[k]))

    files = []
    for idx, (colors, s, l) in enumerate(decks):
        lines = collections.Counter()
        for n in s:
            lines[n] += 4
        for n in l:
            lines[n] += 4
        # fill: verified cheap spells of the deck's colors up to 36 spells
        pool = [f for key, fs in fillers.items() if set(key) <= set(colors) for f in fs]
        rng.shuffle(pool)
        spells_n = sum(v for k, v in lines.items() if not is_land(cards[k]))
        for f in pool:
            if spells_n >= 36:
                break
            if f not in lines:
                take = min(4, 36 - spells_n)
                lines[f] += take
                spells_n += take
        lands_n = sum(v for k, v in lines.items() if is_land(cards[k]))
        main_colors = colors[:2] if len(colors) >= 2 else (colors + 'W' if colors != 'W' else 'WU')
        basics = max(0, 60 - spells_n - lands_n)
        deck_colors = [c for c in COLORS if c in colors] or list(main_colors)
        for j in range(basics):
            lines[BASIC[deck_colors[j % len(deck_colors)]]] += 1
        name = 'f%03d-%04d-%s' % (r, idx, colors.lower())
        path = os.path.join(decks_dir, name + '.txt')
        with open(path, 'w', encoding='utf-8') as fh:
            fh.write('// verification factory round %d: candidates %s\n' % (r, ', '.join(s + l)))
            for n, v in sorted(lines.items(), key=lambda kv: (is_land(cards[kv[0]]), kv[0])):
                fh.write('%d %s\n' % (v, n))
        files.append(name)

    # pairs: each deck against the next `games_per_deck` decks of another color (by sorted file name, as Java sorts)
    files.sort()
    nd = len(files)
    seeds = []
    for a in range(nd):
        # offsets from 0 to nd - 2 pick b = (a + 1 + offset) % nd: never the deck itself
        for offset in rng.sample(range(nd - 1), min(args.games_per_deck, nd - 1)):
            seeds.append(offset * nd + a)
    seeds = sorted(set(seeds))
    with open(os.path.join(out, 'seeds.txt'), 'w') as fh:
        fh.write(','.join(map(str, seeds)) + '\n')
    with open(os.path.join(out, 'candidates.txt'), 'w', encoding='utf-8') as fh:
        fh.write('\n'.join(names) + '\n')
    print('round %d: %d candidates, %d decks, %d games -> %s' % (r, len(names), nd, len(seeds), out), file=sys.stderr)
    return r


# ---------------------------------------------------------------------------------------------------------- record

def record(args):
    r = args.round or rounds(args.work)[-1]
    out = round_dir(args.work, r)
    seeds = open(os.path.join(out, 'seeds.txt')).read().strip().split(',')
    rec = os.path.join(out, 'rec')
    os.makedirs(rec, exist_ok=True)
    done = {int(re.sub(r'\D', '', os.path.basename(f))) for f in glob.glob(os.path.join(rec, 'game-*.json'))}
    seeds = [s for s in seeds if s and int(s) not in done]
    if not seeds:
        print('round %d: every game is recorded' % r, file=sys.stderr)
        return
    procs = max(1, min(args.procs, len(seeds)))
    chunks = [seeds[i::procs] for i in range(procs)]
    log4j = os.path.join(ROOT, '.travis', 'log4j.properties')
    running = []
    for i, chunk in enumerate(chunks):
        cmd = ['mvn', '-o', '-q', '-pl', 'Mage.Tests', 'test', '-Dtest=ShadowGamesTest',
               '-Dsurefire.failIfNoSpecifiedTests=false',
               '-Dxmage.shadowGames=' + rec, '-Dxmage.shadowGames.seeds=' + ','.join(chunk),
               '-Dxmage.shadowGames.turns=%d' % args.turns,
               '-Dxmage.shadowGames.decks=' + os.path.join(out, 'decks'),
               '-Dxmage.dataCollectors.printGameLogs=false', '-Dlog4j.configuration=file:' + log4j]
        logf = open(os.path.join(out, 'record-%02d.log' % i), 'w')
        running.append((subprocess.Popen(cmd, cwd=ROOT, stdout=logf, stderr=subprocess.STDOUT), logf))
        if i == 0 and procs > 1:
            # the first one compiles if anything is stale; the rest start once it's past that
            time.sleep(args.stagger)
    bad = 0
    for p, logf in running:
        bad += p.wait() != 0
        logf.close()
    n = len(glob.glob(os.path.join(rec, 'game-*.json')))
    print('round %d: %d games recorded (%d processes failed)' % (r, n, bad), file=sys.stderr)


# ---------------------------------------------------------------------------------------------------------- replay

GAME_LINE = re.compile(r'^(game-\d+\.json): (matches|(\d+) of (\d+) entries match, (\d+) answers used)')


def replay(args):
    todo = rounds(args.work) if args.all else [args.round or rounds(args.work)[-1]]
    for r in todo:
        out = round_dir(args.work, r)
        rec = os.path.join(out, 'rec')
        env = dict(os.environ, PLAYMAT_SHADOW=rec)
        env.pop('PLAYMAT_SHADOW_STRICT', None)
        cmd = ['cargo', '+stable', 'test', '--release', '-q', '-p', 'playmat-engine', '--test', 'shadow', '--',
               '--nocapture']
        p = subprocess.run(cmd, cwd=os.path.join(ROOT, 'rust'), env=env, capture_output=True, text=True)
        results = parse_replay(p.stderr)
        with open(os.path.join(out, 'replay.json'), 'w') as fh:
            json.dump(results, fh, indent=0)
        m = sum(1 for v in results.values() if v['match'])
        print('round %d: %d of %d replayed games match' % (r, m, len(results)), file=sys.stderr)
        if p.returncode != 0 and not results:
            print(p.stderr[-3000:], file=sys.stderr)


def parse_replay(text):
    """the shadow test's report, per game: matched, or where it parted (the failure, with both texts)"""
    out, cur = {}, None
    for line in text.split('\n'):
        m = GAME_LINE.match(line)
        if m:
            cur = m.group(1)
            if m.group(2) == 'matches':
                out[cur] = {'match': True}
                cur = None
            else:
                out[cur] = {'match': False, 'matched': int(m.group(3)), 'entries': int(m.group(4)),
                            'answers': int(m.group(5)), 'failure': ''}
            continue
        if cur and re.match(r'^\d+ match, \d+ differ, \d+ skipped', line):
            cur = None
            continue
        if cur:
            out[cur]['failure'] += line + '\n'
    return out


# ---------------------------------------------------------------------------------------------------------- verdicts

STACK = re.compile(r'^stack \d+ (.*)$')
BF = re.compile(r'^bf \d+ (.+?) [TU](?: facedown)? ')


def ability_sources(names, facts):
    """a stack ability's text (`stack ability (<rule>)`) -> the cards of the game with that rule"""
    rules = collections.defaultdict(set)
    for n in names:
        for a in facts.get(n, []):
            if a.get('rule'):
                # the facts name the card; a stack ability's text keeps `{this}`
                rules[a['rule']].add(n)
                rules[a['rule'].replace(n, '{this}')].add(n)
    return rules


def stack_names(text, names, rules):
    """the cards on the stack in a trace text, top first: spells by name, abilities by their rule's card"""
    out = []
    for line in (text or '').split('\n'):
        m = STACK.match(line)
        if not m:
            continue
        s = m.group(1)
        if s in names:
            out.append(s)
        elif s.startswith('stack ability (') and s.endswith(')'):
            rule = s[len('stack ability ('):-1]
            src = rules.get(rule)
            if src:
                out.extend(sorted(src))
            else:
                out.extend(n for n in names if n in rule)
    return out


def bf_names(text):
    return {m.group(1) for m in (BF.match(l) for l in (text or '').split('\n')) if m}


def game_cards(g):
    """the game's card names: its libraries and hands at the start (the decks' lists carry a count prefix)"""
    return {n for seat in g['start']['seats'] for n in seat.get('library', []) + seat.get('hand', [])}


def blame(g, res, candidates, names, rules):
    """the first unverified card involved where Java and Rust part"""
    trace = g['trace']
    i = res.get('matched', 0)
    fail = res.get('failure', '')
    m = re.search(r'--- Java\n(.*?)--- Rust\n(.*)', fail, re.S)
    java_text, rust_text = (m.group(1), m.group(2)) if m else (None, None)
    if java_text is None:
        # a question that differs: the first trace entry after the answer it names
        a = re.search(r'answer (\d+)', fail)
        if a:
            k = int(a.group(1))
            later = [j for j, e in enumerate(trace) if (e.get('decision') or 0) >= k]
            i = later[0] if later else len(trace) - 1
        java_text = trace[i]['text'] if i < len(trace) else ''
    order = []
    for t in (java_text, trace[i - 1]['text'] if 0 < i <= len(trace) else ''):
        order += [n for n in stack_names(t, names, rules) if n in candidates]
    if rust_text is not None:
        diff = set(java_text.split('\n')) ^ set(rust_text.split('\n'))
        for line in sorted(diff):
            order += [n for n in candidates if n in line]
    for j in range(min(i, len(trace)) - 1, -1, -1):
        order += [n for n in stack_names(trace[j].get('text'), names, rules) if n in candidates]
        if order:
            break
    return (order[0] if order else '?'), i, java_text, rust_text


def summarize_failure(fail):
    first = fail.strip().split('\n')[0] if fail else ''
    return first[:300]


def verdicts(args):
    cards = load_cards()
    verified = load_verified()
    facts = {c['name']: c.get('abilities', []) for c in json.load(open(FACTS))}
    stats = collections.defaultdict(lambda: {'games': 0, 'exercised': 0, 'blamed': [], 'rounds': set()})
    bugs = collections.defaultdict(list)
    totals = collections.Counter()
    for r in rounds(args.work):
        out = round_dir(args.work, r)
        path = os.path.join(out, 'replay.json')
        if not os.path.exists(path):
            continue
        results = json.load(open(path))
        for n in open(os.path.join(out, 'candidates.txt'), encoding='utf-8').read().split('\n'):
            if n:
                stats[n]['rounds'].add(r)
        for f in sorted(glob.glob(os.path.join(out, 'rec', 'game-*.json'))):
            g = json.load(open(f))
            base = os.path.basename(f)
            names = game_cards(g)
            candidates = {n for n in names if n not in verified}
            if g['result'] == 'timeout' or g['result'].startswith('error'):
                totals['java_unfinished'] += 1
                continue
            res = results.get(base)
            if res is None:
                totals['skipped'] += 1
                continue
            for n in candidates:
                stats[n]['games'] += 1
            rules = ability_sources(names, facts)
            if res['match']:
                totals['match'] += 1
                seen = set()
                for e in g['trace']:
                    seen.update(stack_names(e.get('text'), names, rules))
                    seen.update(n for n in bf_names(e.get('text')) if n in names and is_land(cards.get(n, {})))
                for n in candidates & seen:
                    stats[n]['exercised'] += 1
            else:
                totals['differ'] += 1
                who, at, jt, rt = blame(g, res, candidates, names, rules)
                if who in stats or who == '?':
                    stats[who]['blamed'].append('%d/%s' % (r, base))
                diff = []
                if jt is not None and rt is not None:
                    j, k = set(jt.split('\n')), set(rt.split('\n'))
                    diff = ['Java ' + l for l in jt.split('\n') if l and l not in k][:4] + \
                           ['Rust ' + l for l in rt.split('\n') if l and l not in j][:4]
                bugs[who].append({'game': os.path.relpath(f, args.work), 'entry': at,
                                  'why': summarize_failure(res.get('failure', '')), 'diff': diff})
    newly = []
    rows = []
    for n, s in stats.items():
        if n == '?':
            continue
        if n in verified:
            v = 'verified'
        elif s['blamed']:
            v = 'bug'
        elif s['exercised'] >= args.min_games:
            v = 'verified'
            newly.append(n)
        else:
            v = 'pending'
        rows.append((n, v, s['games'], s['exercised'], len(s['blamed']), len(s['rounds'])))
    rows.sort(key=lambda x: ({'bug': 0, 'pending': 1, 'verified': 2}[x[1]], -x[4], x[0]))
    with open(os.path.join(args.work, 'verdicts.tsv'), 'w', encoding='utf-8') as fh:
        fh.write('card\tverdict\tgames\texercised\tblamed\trounds\n')
        for row in rows:
            fh.write('\t'.join(map(str, row)) + '\n')
    with open(os.path.join(args.work, 'bugs.md'), 'w', encoding='utf-8') as fh:
        fh.write('# Factory mismatches by the first card involved\n\n')
        for who, games in sorted(bugs.items(), key=lambda kv: -len(kv[1])):
            fh.write('## %s (%d game%s)\n\n' % (who, len(games), '' if len(games) == 1 else 's'))
            for b in games[:8]:
                fh.write('- %s, trace entry %s: %s\n' % (b['game'], b['entry'], b['why']))
                for d in b['diff']:
                    fh.write('  - `%s`\n' % d[:200])
            fh.write('\n')
    c = collections.Counter(r[1] for r in rows)
    print('games: %d match, %d differ, %d Java unfinished, %d skipped; cards: %d newly verified, %d bug, %d pending'
          % (totals['match'], totals['differ'], totals['java_unfinished'], totals['skipped'], len(newly), c['bug'],
             c['pending']), file=sys.stderr)
    if args.write and newly:
        allv = sorted(load_verified() | set(newly))
        with open(VERIFIED, 'w', encoding='utf-8') as fh:
            fh.write('\n'.join(allv) + '\n')
        print('cards/verified.txt: %d cards' % len(allv), file=sys.stderr)


def views(args):
    sys.path.insert(0, os.path.dirname(__file__))
    import transpile
    names = sorted(load_verified())
    tmp = os.path.join(args.work, 'views')
    os.makedirs(tmp, exist_ok=True)
    pool = os.path.join(tmp, 'pool.txt')
    with open(pool, 'w', encoding='utf-8') as fh:
        fh.write('\n'.join(names + ['token:' + t for t in transpile.token_classes(names)]
                           + ['emblem:' + e for e in transpile.emblem_classes(names)]) + '\n')
    facts, full = os.path.join(tmp, 'facts.json'), os.path.join(tmp, 'views-full.json')
    log4j = os.path.join(ROOT, '.travis', 'log4j.properties')
    subprocess.run(['mvn', '-o', '-q', '-pl', 'Mage.Tests', 'test', '-Dtest=CardFactsTest',
                    '-Dsurefire.failIfNoSpecifiedTests=false', '-Dxmage.cardFacts=' + facts,
                    '-Dxmage.cardFacts.pool=' + pool, '-Dxmage.cardFacts.views=' + full,
                    '-Dlog4j.configuration=file:' + log4j], cwd=ROOT, check=True)
    subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), 'view_templates.py'), full, facts],
                   check=True)


def main():
    p = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    p.add_argument('command', choices=['build', 'record', 'replay', 'verdicts', 'run', 'views'])
    p.add_argument('--work', required=True, help='the factory folder (decks, recordings, verdicts)')
    p.add_argument('--round', type=int)
    p.add_argument('--all', action='store_true', help='replay: every round')
    p.add_argument('--candidates', help='a file of card names, one per line, instead of every unverified card')
    p.add_argument('--priority', help='a TSV with card and score columns that orders the candidates')
    p.add_argument('--used-only', action='store_true', help='only candidates with a score above 0')
    p.add_argument('--limit', type=int, default=0)
    p.add_argument('--per-deck', type=int, default=6, help='candidate spells per deck')
    p.add_argument('--lands-per-deck', type=int, default=2, help='candidate lands per deck')
    p.add_argument('--decks-per-card', type=int, default=3)
    p.add_argument('--games-per-deck', type=int, default=2)
    p.add_argument('--max-tries', type=int, default=3, help='rounds a candidate may take before it is left pending')
    p.add_argument('--seed', type=int, default=20261112)
    p.add_argument('--turns', type=int, default=20)
    p.add_argument('--procs', type=int, default=8)
    p.add_argument('--stagger', type=int, default=60, help='seconds between the first recorder and the rest')
    p.add_argument('--min-games', type=int, default=3)
    p.add_argument('--write', action='store_true', help='verdicts: add newly verified cards to cards/verified.txt')
    args = p.parse_args()
    os.makedirs(args.work, exist_ok=True)
    if args.command == 'build':
        build(args)
    elif args.command == 'record':
        record(args)
    elif args.command == 'replay':
        replay(args)
    elif args.command == 'verdicts':
        verdicts(args)
    elif args.command == 'views':
        views(args)
    else:
        r = build(args)
        if r is None:
            return
        args.round = r
        record(args)
        replay(args)
        verdicts(args)


if __name__ == '__main__':
    main()

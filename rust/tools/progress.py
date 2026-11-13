#!/usr/bin/env python3
"""Writes site/rust/progress.json, the data behind the site's Rust tracker (site/rust/).

The hand-kept part (phases, steps, benchmark numbers) comes from rust/tools/progress-plan.json; this adds what can be
counted from the repository: cards in the engine and how many of them are verified against Java, the decks the
verified ones cover, the shadow games that match Java, and the Rust
code itself. The site workflow runs it before each deploy, so the counted numbers follow master.

    python3 rust/tools/progress.py            # writes site/rust/progress.json
"""
import argparse
import glob
import json
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
ENGINE = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine')
OUT = os.path.join(ROOT, 'site', 'rust', 'progress.json')


def path(*parts):
    return os.path.join(ROOT, *parts)


def engine_cards():
    cards = json.load(open(os.path.join(ENGINE, 'cards', 'cards.json')))
    return {c['name'] for c in cards}


def name_list(name):
    """a card list under cards/ (verified.txt, pioneer-pool.txt): one name per line"""
    with open(os.path.join(ENGINE, 'cards', name), encoding='utf-8') as f:
        return {l.strip() for l in f if l.strip() and not l.startswith('#')}


def deck_names(file):
    """the card names of a deck file's main deck (`.txt` school decks and `.dck` meta decks)"""
    names = set()
    for line in open(file, encoding='utf-8'):
        line = line.strip()
        if not line or line.startswith('#') or line.startswith('SB:') or line.startswith('NAME:'):
            continue
        m = re.match(r'(\d+)\s+(?:\[[^\]]*\]\s+)?(.+?)(\s+\(.*)?$', line)
        if m:
            names.add(m.group(2).strip())
    return names


def coverage(files, have, label):
    decks, wanted = [], set()
    for f in sorted(files):
        names = deck_names(f)
        if not names:
            continue
        wanted |= names
        missing = sorted(names - have)
        name = os.path.splitext(os.path.basename(f))[0]
        decks.append({'deck': name, 'cards': len(names), 'missing': len(missing)})
    return {
        'label': label,
        'decks': len(decks),
        'covered': sum(1 for d in decks if d['missing'] == 0),
        'cards': len(wanted),
        'cards_in': len(wanted & have),
        'list': decks,
    }


def tiers():
    """each tier of shadow games: what it taught the engine (Engine.md's table) and how many of its games matched"""
    rows = {}
    for line in open(path('docs', 'rust', 'Engine.md'), encoding='utf-8'):
        m = re.match(r'\|\s*(\d+)\s*\|(.*)\|(.*)\|\s*$', line)
        if not m:
            continue
        n, what, games = int(m.group(1)), m.group(2).strip(), m.group(3).strip()
        what = re.sub(r'^`shadow-decks/tier\d+`:\s*', '', what)
        what = re.sub(r'`([^`]*)`', r'\1', what)
        g = re.search(r'(\d+) of (\d+)', games)
        # a game Java itself didn't finish in time has nothing to compare
        timeouts = re.search(r'(\d+) Java timeouts?', games)
        played = int(g.group(2)) if g else None
        if played is not None and timeouts and 'finished' not in games:
            played -= int(timeouts.group(1))
        rows[n] = {'tier': n, 'what': what, 'matched': int(g.group(1)) if g else None, 'played': played,
                   'timeouts': int(timeouts.group(1)) if timeouts else 0, 'note': games}
    out = []
    dirs = glob.glob(os.path.join(ENGINE, 'shadow', 'tier*'))
    for d in sorted(dirs, key=lambda d: int(re.sub(r'\D', '', os.path.basename(d)) or 0)):
        n = int(re.sub(r'\D', '', os.path.basename(d)) or 0)
        row = rows.get(n, {'tier': n, 'what': '', 'matched': None, 'played': None, 'timeouts': 0, 'note': ''})
        row['checked_in'] = len(glob.glob(os.path.join(d, '*.json')))
        decks = glob.glob(os.path.join(ENGINE, 'shadow-decks', 'tier%d' % n, '*.txt'))
        row['decks'] = len(decks)
        out.append(row)
    return out


def rust_lines():
    total, crates = 0, []
    for crate in sorted(glob.glob(path('rust', 'crates', '*'))):
        n = 0
        for f in glob.glob(os.path.join(crate, '**', '*.rs'), recursive=True):
            with open(f, encoding='utf-8', errors='replace') as h:
                n += sum(1 for _ in h)
        crates.append({'crate': os.path.basename(crate), 'lines': n})
        total += n
    return {'lines': total, 'crates': crates}


def java_card_files():
    return len(glob.glob(path('Mage.Sets', 'src', 'mage', 'cards', '*', '*.java')))


def commit():
    try:
        sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        date = subprocess.check_output(['git', 'log', '-1', '--format=%cs'], cwd=ROOT, text=True).strip()
        return sha, date
    except (OSError, subprocess.CalledProcessError):
        return None, None


def build():
    plan = json.load(open(path('rust', 'tools', 'progress-plan.json'), encoding='utf-8'))
    plan.pop('_about', None)
    in_engine = engine_cards()
    # a game goes to Rust only with verified cards, so the decks count as covered by those
    have = name_list('verified.txt') & in_engine
    pioneer = name_list('pioneer-pool.txt')
    school = glob.glob(path('Mage.Server', 'career-src', 'pioneer', '*.txt')) + \
        glob.glob(path('Mage.Server', 'career-src', 'standard', '*.txt'))
    meta = [f for f in glob.glob(path('Mage.Web.Client', 'public', 'meta-decks', '*.dck'))
            if re.match(r'(pioneer|standard)-', os.path.basename(f))]
    t = tiers()
    played = sum(r['played'] or 0 for r in t)
    matched = sum(r['matched'] or 0 for r in t)
    timeouts = sum(r['timeouts'] for r in t)
    return dict(plan, counted={
        'engine_cards': len(in_engine),
        'verified_cards': len(have),
        'pioneer': {'cards': len(pioneer), 'in_engine': len(pioneer & in_engine), 'verified': len(pioneer & have)},
        'java_card_files': java_card_files(),
        'school': coverage(school, have, 'Pioneer and Standard school decks'),
        'meta': coverage(meta, have, 'Pioneer and Standard meta decks'),
        'tiers': t,
        'shadow': {'played': played, 'matched': matched, 'java_timeouts': timeouts,
                   # every game under shadow/ is replayed by the tests: the tiers' and the gate run's (gate10k)
                   'checked_in': len(glob.glob(os.path.join(ENGINE, 'shadow', '*', '*.json')))},
        'rust': rust_lines(),
    })


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', default=OUT)
    args = parser.parse_args()
    data = build()
    sha, date = commit()
    data['commit'] = sha
    data['generated'] = date
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=1, ensure_ascii=False)
        f.write('\n')
    c = data['counted']
    print('%d engine cards, %d verified; school decks %d of %d; meta decks %d of %d; shadow games %d of %d match' % (
        c['engine_cards'], c['verified_cards'], c['school']['covered'], c['school']['decks'], c['meta']['covered'], c['meta']['decks'],
        c['shadow']['matched'], c['shadow']['played']), file=sys.stderr)


if __name__ == '__main__':
    main()

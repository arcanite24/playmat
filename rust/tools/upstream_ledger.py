#!/usr/bin/env python3
"""The upstream ledger (#191, docs/rust/Upstream.md): what upstream XMage changed in cards and the engine since the
Rust cards were last translated, and what each change means for Rust.

    python3 rust/tools/upstream_ledger.py scan --from <upstream sha> --to <upstream sha>
        records every card and engine file the range changed in rust/upstream/ledger.toml
    python3 rust/tools/upstream_ledger.py check --facts <facts.json>
        re-translates the changed cards in the engine with the facts Java writes for them now (CardFactsTest, below):
        a card whose translation is the same is `ported` (nothing for Rust to do), one that differs or no longer
        translates is `pending`; the verified pending cards go to cards/upstream-pending.txt, which the engine's
        gate leaves out (a card upstream changed plays on Java until it's re-verified)
    python3 rust/tools/upstream_ledger.py requeue
        after the cards are translated again (transpile.py, docs/rust/Engine.md): takes the pending cards off
        verified.txt and upstream-pending.txt, so the factory (verify_factory.py) verifies them again
    python3 rust/tools/upstream_ledger.py facts-pool <file>
        writes the CardFactsTest pool for `check`: the changed cards in the engine's pool, and their tokens

The facts for `check`:

    python3 rust/tools/upstream_ledger.py facts-pool /tmp/upstream-pool.txt
    mvn -o -q -pl Mage.Tests test -Dtest=CardFactsTest -Dsurefire.failIfNoSpecifiedTests=false \\
      -Dxmage.cardFacts=/tmp/upstream-facts.json -Dxmage.cardFacts.pool=/tmp/upstream-pool.txt
    python3 rust/tools/upstream_ledger.py check --facts /tmp/upstream-facts.json
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
LEDGER = os.path.join(ROOT, 'rust', 'upstream', 'ledger.toml')
CARDS = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'cards')
PENDING = os.path.join(CARDS, 'upstream-pending.txt')
CARD_DIR = 'Mage.Sets/src/mage/cards/'
ENGINE_DIR = 'Mage/src/main/java/'


def git(*args):
    return subprocess.run(['git', '-C', ROOT] + list(args), check=True, capture_output=True, text=True).stdout


def names_by_class():
    """each card class (mage.cards.a.AcademyDrake) and the names the sets print it under"""
    found = {}
    sets = os.path.join(ROOT, 'Mage.Sets', 'src', 'mage', 'sets')
    pattern = re.compile(r'SetCardInfo\("((?:[^"\\]|\\.)*)",[^;]*?(mage\.cards\.[a-z0-9]\.\w+)\.class')
    for f in os.listdir(sets):
        if f.endswith('.java'):
            for name, cls in pattern.findall(open(os.path.join(sets, f), encoding='utf-8', errors='ignore').read()):
                found.setdefault(cls, set()).add(name.replace('\\"', '"'))
    return found


def lines(path):
    if not os.path.exists(path):
        return set()
    return {l.strip() for l in open(path, encoding='utf-8') if l.strip() and not l.startswith('#')}


def engine_pool():
    """the engine's pool: Pioneer, Career, and server-pool cards outside them (transpile.engine_pool)"""
    return (lines(os.path.join(CARDS, 'pioneer-pool.txt'))
            | lines(os.path.join(CARDS, 'career-pool.txt'))
            | lines(os.path.join(CARDS, 'server-pool.txt')))


# --- the ledger file: a small TOML of our own shape, written and read here ---------------------------------------

def quote(s):
    return '"' + str(s).replace('\\', '\\\\').replace('"', '\\"') + '"'


def write_ledger(meta, entries):
    os.makedirs(os.path.dirname(LEDGER), exist_ok=True)
    out = ['# The upstream ledger (docs/rust/Upstream.md): written by rust/tools/upstream_ledger.py, edit `status` and',
           '# `pr` by hand only for engine entries (cards are judged by `check`).', '']
    for k in ('synced', 'from', 'to', 'checked'):
        if meta.get(k):
            out.append('%s = %s' % (k, quote(meta[k])))
    for e in entries:
        out += ['', '[[change]]']
        for k in ('file', 'kind', 'class', 'names', 'scope', 'status', 'commit', 'subject', 'why', 'pr'):
            v = e.get(k)
            if v is None or v == '' or v == []:
                continue
            out.append('%s = %s' % (k, '[' + ', '.join(quote(x) for x in v) + ']' if isinstance(v, list) else quote(v)))
    with open(LEDGER, 'w', encoding='utf-8') as f:
        f.write('\n'.join(out) + '\n')


def read_ledger():
    try:
        import tomllib
    except ImportError:  # Python < 3.11
        sys.exit('needs Python 3.11 or later (tomllib)')
    with open(LEDGER, 'rb') as f:
        data = tomllib.load(f)
    return {k: v for k, v in data.items() if k != 'change'}, data.get('change', [])


# --- scan ---------------------------------------------------------------------------------------------------------

def scan(base, head):
    pool = engine_pool()
    verified = lines(os.path.join(CARDS, 'verified.txt'))
    in_engine = {c['name'] for c in json.load(open(os.path.join(CARDS, 'cards.json')))}
    by_class = names_by_class()
    entries = []
    status = git('diff', '--name-status', base, head, '--', CARD_DIR, ENGINE_DIR)
    for row in status.splitlines():
        parts = row.split('\t')
        change, path = parts[0][0], parts[-1]
        if not path.endswith('.java'):
            continue
        log = git('log', '-1', '--format=%h%x09%s', base + '..' + head, '--', path).strip().split('\t', 1)
        commit, subject = (log + ['', ''])[:2]
        entry = {'file': path, 'commit': commit, 'subject': subject[:120]}
        if path.startswith(CARD_DIR):
            cls = path[len('Mage.Sets/src/'):-len('.java')].replace('/', '.')
            names = sorted(by_class.get(cls, set()))
            entry.update({'kind': {'A': 'new card', 'D': 'removed card'}.get(change, 'changed card'),
                          'class': cls, 'names': names})
            if any(n in verified for n in names):
                entry['scope'] = 'verified'
            elif any(n in in_engine for n in names):
                entry['scope'] = 'engine'
            elif any(n in pool for n in names):
                entry['scope'] = 'pool'
            else:
                entry['scope'] = 'outside'
            # a card outside the engine's pool waits for its format (#191's scope); one in it is judged by `check`
            entry['status'] = 'pending' if entry['scope'] != 'outside' else 'not-needed'
            if entry['scope'] == 'outside':
                entry['why'] = 'outside the engine pool (Pioneer and Career): recorded, not ported'
        else:
            cls = path[len(ENGINE_DIR):-len('.java')].replace('/', '.')
            entry.update({'kind': 'engine', 'class': cls, 'scope': 'engine', 'status': 'pending'})
        entries.append(entry)
    entries.sort(key=lambda e: ({'verified': 0, 'engine': 1, 'pool': 2, 'outside': 3}[e['scope']], e['kind'], e['file']))
    write_ledger({'from': base, 'to': head, 'synced': head}, entries)
    counts = {}
    for e in entries:
        key = (e['kind'], e['scope'])
        counts[key] = counts.get(key, 0) + 1
    for (kind, scope), n in sorted(counts.items()):
        print('%5d %s (%s)' % (n, kind, scope))
    print('wrote', os.path.relpath(LEDGER, ROOT))


# --- check --------------------------------------------------------------------------------------------------------

def changed_engine_cards(entries):
    """the names of changed and new cards in the engine's pool, which `check` judges"""
    pool = engine_pool()
    names = set()
    for e in entries:
        if e['kind'] in ('changed card', 'new card') and e['scope'] != 'outside':
            names.update(n for n in e.get('names', []) if n in pool)
    return sorted(names)


def facts_pool(out):
    _, entries = read_ledger()
    names = changed_engine_cards(entries)
    with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as f:
        f.write('\n'.join(names) + '\n')
    subprocess.run([sys.executable, os.path.join(ROOT, 'rust', 'tools', 'transpile.py'), '--pool', f.name,
                    '--write-facts-pool', out], check=True, cwd=ROOT)
    print('%d cards (and their tokens) in %s' % (len(names), out))


def check(facts):
    meta, entries = read_ledger()
    names = changed_engine_cards(entries)
    current = {c['name']: c for c in json.load(open(os.path.join(CARDS, 'cards.json')))}
    verified = lines(os.path.join(CARDS, 'verified.txt'))
    old_facts = {c['name']: c for c in json.load(open(os.path.join(CARDS, 'facts.json')))}
    new_facts = {c['name']: c for c in json.load(open(facts))}
    # the transpiler reads token and emblem facts too: the engine's own, with the changed ones over them
    merged = dict(old_facts)
    merged.update(new_facts)
    work = tempfile.mkdtemp(prefix='upstream-ledger-')
    with open(os.path.join(work, 'facts.json'), 'w') as f:
        json.dump(list(merged.values()), f)
    with open(os.path.join(work, 'pool.txt'), 'w') as f:
        f.write('\n'.join(names) + '\n')
    subprocess.run([sys.executable, os.path.join(ROOT, 'rust', 'tools', 'transpile.py'), '--pool',
                    os.path.join(work, 'pool.txt'), '--facts', os.path.join(work, 'facts.json'), '--out',
                    os.path.join(work, 'cards.json'), '--report', os.path.join(work, 'report.txt')],
                   check=True, cwd=ROOT, stdout=subprocess.DEVNULL)
    now = {c['name']: c for c in json.load(open(os.path.join(work, 'cards.json')))}
    reasons = {}
    for l in open(os.path.join(work, 'report.txt'), encoding='utf-8'):
        if ': ' in l:
            n, why = l.rstrip('\n').split(': ', 1)
            reasons[n] = why
    verdict = {}
    for n in names:
        if n not in now:
            verdict[n] = ('pending', 'no longer translates: ' + reasons.get(n, '?')) if n in current else (
                'not-needed', 'not in the engine before or now: ' + reasons.get(n, '?'))
        elif n not in current:
            verdict[n] = ('pending', 'new to the engine: verify it with the factory')
        elif now[n] == current[n]:
            verdict[n] = ('ported', 'the same translation')
        else:
            verdict[n] = ('pending', 'the translation changed: re-transpile and re-verify')
    pending_verified = set()
    for e in entries:
        own = [n for n in e.get('names', []) if n in verdict]
        if not own or e['kind'] not in ('changed card', 'new card'):
            continue
        states = [verdict[n] for n in own]
        pend = [s for s in states if s[0] == 'pending']
        e['status'], e['why'] = pend[0] if pend else states[0]
        pending_verified.update(n for n in own if verdict[n][0] == 'pending' and n in verified)
    meta['checked'] = meta.get('to', '')
    write_ledger(meta, entries)
    with open(PENDING, 'w', encoding='utf-8') as f:
        f.write('# Verified cards upstream changed and Rust hasn\'t caught up with (rust/tools/upstream_ledger.py check):\n'
                '# the engine\'s gate leaves them out, so their games stay on Java until they\'re re-verified.\n')
        f.write(''.join(n + '\n' for n in sorted(pending_verified)))
    tally = {}
    for status, _ in verdict.values():
        tally[status] = tally.get(status, 0) + 1
    print('%d changed cards in the engine pool: %s' % (len(names), ', '.join('%d %s' % (v, k) for k, v in sorted(tally.items()))))
    print('%d verified cards now wait for a re-check (%s)' % (len(pending_verified), os.path.relpath(PENDING, ROOT)))
    print('facts changed for %d of them' % sum(1 for n in names if old_facts.get(n) != new_facts.get(n)))


def requeue():
    pending = lines(PENDING)
    verified_path = os.path.join(CARDS, 'verified.txt')
    kept = [l for l in open(verified_path, encoding='utf-8') if l.strip() not in pending]
    with open(verified_path, 'w', encoding='utf-8') as f:
        f.writelines(kept)
    header = [l for l in open(PENDING, encoding='utf-8') if l.startswith('#')]
    with open(PENDING, 'w', encoding='utf-8') as f:
        f.writelines(header)
    print('%d cards back to the factory; verified.txt has %d' % (len(pending), sum(1 for l in kept if l.strip())))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest='cmd', required=True)
    s = sub.add_parser('scan')
    s.add_argument('--from', dest='base', required=True)
    s.add_argument('--to', dest='head', required=True)
    p = sub.add_parser('facts-pool')
    p.add_argument('out')
    c = sub.add_parser('check')
    c.add_argument('--facts', required=True)
    sub.add_parser('requeue')
    args = parser.parse_args()
    if args.cmd == 'scan':
        scan(git('rev-parse', args.base).strip(), git('rev-parse', args.head).strip())
    elif args.cmd == 'facts-pool':
        facts_pool(args.out)
    elif args.cmd == 'requeue':
        requeue()
    else:
        check(args.facts)


if __name__ == '__main__':
    main()

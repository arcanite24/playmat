#!/usr/bin/env python3
"""Writes site/benchmarks/benchmarks.json, the data behind the site's public Benchmarks page (site/benchmarks/).

Combines the curated benchmark records in rust/tools/benchmarks-data.json with live suite counts read directly from
archbench/ (constructed suites, tactical puzzle suites, and constructed-1/results.jsonl), then stamps the repository
commit and date. The site workflow runs this before each GitHub Pages deploy.

    python3 rust/tools/benchmarks.py          # writes site/benchmarks/benchmarks.json
"""
import argparse
import glob
import json
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
ARCHBENCH = os.path.join(ROOT, 'archbench')
DATA_IN = os.path.join(os.path.dirname(__file__), 'benchmarks-data.json')
OUT = os.path.join(ROOT, 'site', 'benchmarks', 'benchmarks.json')


def scan_suites():
    suites = []
    for path in sorted(glob.glob(os.path.join(ARCHBENCH, '*', 'suite.json'))):
        d = json.load(open(path, encoding='utf-8'))
        folder = os.path.basename(os.path.dirname(path))
        suites.append({
            'id': d.get('suite', folder),
            'folder': folder,
            'kind': 'duels',
            'created': d.get('created'),
            'decks': len(d.get('decks', [])),
            'deals': d.get('deals'),
            'turn_cap': d.get('turn_cap'),
            'mulligans': bool(d.get('mulligans', False)),
            'duplicate': bool(d.get('duplicate', True)),
            'about': d.get('about', ''),
        })
    for path in sorted(glob.glob(os.path.join(ARCHBENCH, '*', 'puzzles.json'))):
        d = json.load(open(path, encoding='utf-8'))
        folder = os.path.basename(os.path.dirname(path))
        puzzles = d.get('puzzles', [])
        cats = sorted({p['category'] for p in puzzles if 'category' in p})
        suites.append({
            'id': d.get('suite', folder),
            'folder': folder,
            'kind': 'puzzles',
            'created': d.get('created'),
            'puzzles': len(puzzles),
            'categories': cats,
            'about': d.get('about', ''),
        })
    return suites


def read_constructed1_jsonl():
    path = os.path.join(ARCHBENCH, 'constructed-1', 'results.jsonl')
    if not os.path.exists(path):
        return []
    out = []
    with open(path, encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def commit():
    try:
        sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        date = subprocess.check_output(['git', 'log', '-1', '--format=%cs'], cwd=ROOT, text=True).strip()
        return sha, date
    except (OSError, subprocess.CalledProcessError):
        return None, None


def build():
    data = json.load(open(DATA_IN, encoding='utf-8'))
    data.pop('_about', None)
    suites = scan_suites()
    c1 = read_constructed1_jsonl()
    data['counted'] = {
        'suites': suites,
        'duel_suites': sum(1 for s in suites if s['kind'] == 'duels'),
        'puzzle_suites': sum(1 for s in suites if s['kind'] == 'puzzles'),
        'total_puzzles': sum(s.get('puzzles', 0) for s in suites if s['kind'] == 'puzzles'),
        'constructed1_runs': c1,
    }
    sha, date = commit()
    data['commit'] = sha
    data['generated'] = date
    return data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', default=OUT)
    args = parser.parse_args()
    data = build()
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=1, ensure_ascii=False)
        f.write('\n')
    c = data['counted']
    print(
        '%d Archbench suites (%d duel, %d puzzle / %d puzzles); %d combined gauntlet models; %d external engines; %d LLMs'
        % (
            len(c['suites']),
            c['duel_suites'],
            c['puzzle_suites'],
            c['total_puzzles'],
            len(data['combined_gauntlet']['entries']),
            len(data['external_engines']['entries']),
            len(data['llm_leaderboard']['entries']),
        ),
        file=sys.stderr,
    )


if __name__ == '__main__':
    main()

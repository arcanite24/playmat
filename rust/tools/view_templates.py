#!/usr/bin/env python3
"""Writes cards/views.json, the templates the engine's views start from (docs/rust/Views.md), out of what
CardFactsTest writes with -Dxmage.cardFacts.views: each card's view by name, each face's permanent view
("permanent:<name>") and each token class's view and images ("token:<class>"), with ids relative to the object's
own ("$", "$f0", the owner's "$P"; "?" for one the engine can't place), which the engine fills in; and each card's
abilities' rules ("abilities:<name>"), from the facts.

    python3 rust/tools/view_templates.py <views-full.json> <facts-full.json>
"""
import json
import os
import sys

def main():
    views = json.load(open(sys.argv[1]))
    facts = {x['name']: x for x in json.load(open(sys.argv[2]))}
    out = {}
    for name, v in views.items():
        if name.startswith('token:'):
            view = dict(v['view'])
            # a token's rules are its abilities' (`CardView(token)` leaves them out)
            view['rules'] = [a['rule'] for a in facts.get(name, {}).get('abilities', []) if a.get('rule')]
            out[name] = {'view': view, 'images': v['images']}
        else:
            out[name] = v
    # each card's and token's abilities' rules, in Java's order (the engine's), for abilities on the stack
    for name, f in facts.items():
        out['abilities:' + name] = [a.get('rule', '') for a in f.get('abilities', [])]
    path = os.path.join(os.path.dirname(__file__), '..', 'crates', 'playmat-engine', 'cards', 'views.json')
    with open(path, 'w') as f:
        json.dump(out, f, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
        f.write('\n')
    print(len(out), 'views')


if __name__ == '__main__':
    main()

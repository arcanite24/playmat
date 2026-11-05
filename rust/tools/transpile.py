#!/usr/bin/env python3
"""Cards for the Rust engine (docs/rust/Engine.md): reads card classes in Mage.Sets and writes the ones the engine
can express as JSON, one object per card, named after the Java framework classes they use.

    python3 rust/tools/transpile.py [--pool FILE] [--out FILE] [--report FILE]

--pool names the cards to translate, one per line (default: every card in the Pioneer and Standard Career school
decks). A card statement the translator doesn't know makes the card unsupported; --report lists why, per card.
"""
import argparse
import glob
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SETS = os.path.join(ROOT, 'Mage.Sets', 'src', 'mage', 'cards')
SUBTYPES = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'constants', 'SubType.java')
DEFAULT_OUT = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'cards', 'cards.json')

BASIC_LANDS = {
    'Plains': 'WhiteManaAbility',
    'Island': 'BlueManaAbility',
    'Swamp': 'BlackManaAbility',
    'Mountain': 'RedManaAbility',
    'Forest': 'GreenManaAbility',
}

# abilities used as X.getInstance() or new X() with no arguments
KEYWORDS = {
    'FlyingAbility', 'ReachAbility', 'HasteAbility', 'VigilanceAbility', 'TrampleAbility', 'FirstStrikeAbility',
    'DoubleStrikeAbility', 'LifelinkAbility', 'DeathtouchAbility', 'DefenderAbility', 'MenaceAbility', 'FlashAbility',
    'HexproofAbility', 'IndestructibleAbility', 'WhiteManaAbility', 'BlueManaAbility', 'BlackManaAbility',
    'RedManaAbility', 'GreenManaAbility', 'ColorlessManaAbility',
}

# effects: class -> names of its constructor's arguments (int constants only, for now)
EFFECTS = {
    'DamageTargetEffect': ['amount'],
    'DrawCardSourceControllerEffect': ['amount'],
    'GainLifeEffect': ['amount'],
}

# targets with no arguments
TARGETS = {'TargetAnyTarget', 'TargetCreaturePermanent', 'TargetPlayer', 'TargetOpponent'}


class Unsupported(Exception):
    pass


def class_name(card_name):
    name = card_name.split(' // ')[0]
    name = re.sub(r"[^A-Za-z0-9 ]", "", name.replace('-', ' '))
    return ''.join(w[:1].upper() + w[1:] for w in name.split())


def subtype_names():
    names = {}
    for m in re.finditer(r'^\s*(\w+)\("([^"]+)"', open(SUBTYPES).read(), re.M):
        names[m.group(1)] = m.group(2)
    return names


def constructor_body(source, cls):
    m = re.search(r'public\s+' + cls + r'\s*\(\s*UUID\s+ownerId\s*,\s*CardSetInfo\s+setInfo\s*\)\s*\{', source)
    if not m:
        raise Unsupported('no card constructor')
    depth, i = 1, m.end()
    while depth:
        c = source[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
        i += 1
    return source[m.end():i - 1]


def statements(body):
    body = re.sub(r'//[^\n]*', '', body)
    body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
    out, depth, start, quote = [], 0, 0, False
    for i, c in enumerate(body):
        if c == '"':
            quote = not quote
        elif quote:
            continue
        elif c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ';' and depth == 0:
            out.append(canonical(body[start:i]))
            start = i + 1
    rest = canonical(body[start:])
    if rest:
        out.append(rest)
    return [s for s in out if s]


def canonical(text):
    """one statement on one line, with no spaces around punctuation: `this.power=new MageInt(2)`"""
    text = ' '.join(text.split())
    return re.sub(r'\s*([(),{}\[\]=])\s*', r'\1', text)


def new_expr(expr):
    """`new X(args)` or `X.getInstance()` -> (X, [args])"""
    expr = expr.strip()
    m = re.fullmatch(r'(\w+)\.getInstance\(\)', expr)
    if m:
        return m.group(1), []
    m = re.fullmatch(r'new (\w+)\((.*)\)', expr)
    if not m:
        raise Unsupported('expression ' + expr)
    args = [a.strip() for a in split_args(m.group(2))] if m.group(2).strip() else []
    return m.group(1), args


def split_args(text):
    out, depth, start = [], 0, 0
    for i, c in enumerate(text):
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ',' and depth == 0:
            out.append(text[start:i])
            start = i + 1
    out.append(text[start:])
    return out


def int_arg(arg):
    if re.fullmatch(r'-?\d+', arg):
        return int(arg)
    raise Unsupported('argument ' + arg)


def effect(expr):
    cls, args = new_expr(expr)
    if cls not in EFFECTS:
        raise Unsupported('effect ' + cls)
    names = EFFECTS[cls]
    if len(args) != len(names):
        raise Unsupported('effect ' + expr)
    out = {'class': cls}
    for name, arg in zip(names, args):
        out[name] = int_arg(arg)
    return out


def target(expr):
    cls, args = new_expr(expr)
    if cls not in TARGETS or args:
        raise Unsupported('target ' + expr)
    return {'class': cls}


def translate(card_name, subtypes):
    if card_name in BASIC_LANDS:
        return {'name': card_name, 'types': ['LAND'], 'supertypes': ['BASIC'], 'subtypes': [card_name],
                'colors': [], 'abilities': [{'class': BASIC_LANDS[card_name]}]}
    cls = class_name(card_name)
    path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no card class ' + cls)
    source = open(path).read()
    if re.search(r'extends (?!CardImpl\b)\w+', re.search(r'public final class \w+ extends \w+', source).group(0)):
        raise Unsupported('card class is not a CardImpl')
    card = {'name': card_name}
    abilities, spell = [], {'effects': [], 'targets': []}
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},(".*?"|null)\)', st)
        if m:
            card['types'] = [t.strip().split('.')[-1] for t in m.group(1).split(',') if t.strip()]
            card['cost'] = '' if m.group(2) == 'null' else m.group(2)[1:-1]
            continue
        m = re.fullmatch(r'this\.subtype\.add\((.*)\)', st)
        if m:
            for s in split_args(m.group(1)):
                key = s.strip().split('.')[-1]
                if key not in subtypes:
                    raise Unsupported('subtype ' + s)
                card.setdefault('subtypes', []).append(subtypes[key])
            continue
        m = re.fullmatch(r'this\.supertype\.add\((.*)\)', st)
        if m:
            card.setdefault('supertypes', []).extend(s.strip().split('.')[-1] for s in split_args(m.group(1)))
            continue
        m = re.fullmatch(r'this\.(power|toughness)=new MageInt\((-?\d+)\)', st)
        if m:
            card[m.group(1)] = int(m.group(2))
            continue
        m = re.fullmatch(r'this\.addAbility\((.*)\)', st)
        if m:
            cls_, args = new_expr(m.group(1))
            if cls_ not in KEYWORDS or args:
                raise Unsupported('ability ' + m.group(1))
            abilities.append({'class': cls_})
            continue
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addEffect\((.*)\)', st)
        if m:
            spell['effects'].append(effect(m.group(1)))
            continue
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addTarget\((.*)\)', st)
        if m:
            spell['targets'].append(target(m.group(1)))
            continue
        raise Unsupported('statement ' + st)
    if 'types' not in card:
        raise Unsupported('no card types')
    if abilities:
        card['abilities'] = abilities
    if spell['effects'] or spell['targets']:
        card['spell'] = spell
    return card


def school_pool():
    names = set()
    for f in glob.glob(os.path.join(ROOT, 'Mage.Server', 'career-src', 'pioneer', '*.txt')) + \
            glob.glob(os.path.join(ROOT, 'Mage.Server', 'career-src', 'standard', '*.txt')) + \
            glob.glob(os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'shadow-decks', '*', '*.txt')):
        for line in open(f):
            line = line.strip()
            m = re.match(r'(?:SB:\s*)?(\d+)\s+(.+?)(\s+\(.*)?$', line)
            if line and not line.startswith('#') and m:
                names.add(m.group(2).strip())
    return sorted(names)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pool')
    parser.add_argument('--out', default=DEFAULT_OUT)
    parser.add_argument('--report')
    args = parser.parse_args()
    pool = [l.strip() for l in open(args.pool)] if args.pool else school_pool()
    subtypes = subtype_names()
    cards, report = [], []
    for name in pool:
        if not name:
            continue
        try:
            cards.append(translate(name, subtypes))
        except Unsupported as e:
            report.append('%s: %s' % (name, e))
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w') as f:
        f.write('[\n' + ',\n'.join(json.dumps(c, sort_keys=True) for c in sorted(cards, key=lambda c: c['name'])) + '\n]\n')
    if args.report:
        with open(args.report, 'w') as f:
            f.write('\n'.join(report) + '\n')
    print('%d of %d cards translated' % (len(cards), len([n for n in pool if n])), file=sys.stderr)


if __name__ == '__main__':
    main()

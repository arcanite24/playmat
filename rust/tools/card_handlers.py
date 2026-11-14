"""Per-card handlers (docs/rust/Engine.md, "Per-card handlers"): the Java classes written by hand in Rust, one file
per card under rust/crates/playmat-engine/src/cards/. transpile.py asks here before it gives up on a class a card
declares for itself, and emits a reference to the handler instead.

A handler file declares its classes as

    pub const HANDLERS: &[Handler] = &[
        Handler::effect("RapidHybridization.RapidHybridizationEffect", apply).tokens(&["FrogLizardToken"]),
    ];

`Handler::effect`, `::condition`, `::value`, `::predicate` or `::trigger` (with the framework trigger it extends, in Java, as its
second argument), with the Java class as `OuterCardClass.InnerClass` (or the bare class
name for one in the framework), and optionally `.tokens(&[…])`, `.effects(&[…])`, `.filters(&[…])`: Java for the
transpiler to translate (token classes, effect and filter expressions), handed to the handler in that order; and
`.args(&[…])`, one kind per constructor argument ("text" as is, "library_target": a `TargetCardInLibrary`, as
"min max" with its filter after the declared ones; "filter", "effect" and "ability": translated, after the declared
filters, effects or abilities; "counter": a `Counter` as "name amount"; "counter_type": `CounterType.X` as the
engine's counter name; "put_cards": a `PutCards` constant's name;
"while_on_battlefield": only `Duration.WhileOnBattlefield`; "one_of:a|b": only one of these texts; "duration": an
optional leading `Duration`, `Duration.WhileOnBattlefield` when the constructor has none). The strings are
plain or raw Rust string literals.

The static kinds (src/card_statics.rs) are `Handler::continuous`, `::replacement`, `::cost`, `::restriction`, `::rule`,
`::as_though` and `::keeps_mana`, with any arguments after the name (no string literals among them), and
`.abilities(&[…])` and `.on_start(f)` too.
A class with one becomes `{"class": "CardStatic", "handler": …}`, wherever an effect can be: in a static ability, it
lasts while its source is on the battlefield; resolving, until the turn's cleanup.
"""
import glob
import os
import re

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
DIR = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'src', 'cards')

STATIC_KINDS = ('continuous', 'replacement', 'cost', 'restriction', 'rule', 'as_though', 'keeps_mana')
_DECL = re.compile(r'Handler::(effect|condition|value|trigger|predicate|mana|' + '|'.join(STATIC_KINDS) + r')\(\s*"([\w.]+)"\s*,')
_FN = re.compile(r'\s*[\w:]+\s*,?\s*\)')
_PART = re.compile(r'\s*\.(tokens|effects|filters|args|abilities)\(\s*&\[')
# `.in_any_zone()` and `.on_start(f)`: the engine's business only
_ANY_ZONE = re.compile(r'\s*\.(?:in_any_zone\(\)|on_start\(\s*[\w:]+\s*\))')
_STR = re.compile(r'\s*(?:"((?:[^"\\]|\\.)*)"|r(#*)"(.*?)"\2)\s*,?', re.S)

_cache = None
# transpile.py, set as it imports this module (for `token`)
T = None
# the card file being translated, set by transpile.py as it reads the card's filters (`predicate`)
SOURCE = None


class Unsupported(Exception):
    pass


def _unsupported(text):
    # transpile.py's own exception, so the card is reported as blocked rather than the run failing
    return (T.Unsupported if T else Unsupported)(text)


def _unescape(s):
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), s)


def _parse(text, path):
    out = {}
    for m in _DECL.finditer(text):
        h = {'kind': m.group(1), 'file': os.path.relpath(path, ROOT), 'tokens': [], 'effects': [], 'filters': [],
             'args': [], 'abilities': [], 'base': None}
        i = m.end()
        if m.group(1) == 'trigger':
            # `Handler::trigger("X", "<framework trigger in Java>", f)`
            b = _STR.match(text, i)
            if not b or not b.group(0).strip():
                raise SyntaxError(f'{path}: Handler::trigger takes its base trigger as a string literal')
            h['base'] = _unescape(b.group(1)) if b.group(1) is not None else b.group(3)
            i = b.end()
        if m.group(1) in STATIC_KINDS or m.group(1) == 'mana':
            # the rest of the arguments, to the closing parenthesis
            depth = 1
            while depth and i < len(text):
                if text[i] == '"':
                    raise SyntaxError(f'{path}: Handler::{m.group(1)}("{m.group(2)}", ...) takes no string literals')
                depth += {'(': 1, ')': -1}.get(text[i], 0)
                i += 1
            if depth:
                raise SyntaxError(f'{path}: Handler::{m.group(1)}("{m.group(2)}", ...) not closed')
        else:
            f = _FN.match(text, i)
            if not f:
                raise SyntaxError(f'{path}: Handler::{m.group(1)}("{m.group(2)}", ...) not understood')
            i = f.end()
        while (p := _PART.match(text, i)) or (z := _ANY_ZONE.match(text, i)):
            if not p:
                # `.in_any_zone()`: the engine's business only
                i = z.end()
                continue
            i = p.end()
            while (s := _STR.match(text, i)) and s.group(0).strip():
                h[p.group(1)].append(_unescape(s.group(1)) if s.group(1) is not None else s.group(3))
                i = s.end()
            close = re.compile(r'\s*\]\s*\)').match(text, i)
            if not close:
                raise SyntaxError(f'{path}: .{p.group(1)}(&[…]) takes string literals only')
            i = close.end()
        if m.group(2) in out:
            raise SyntaxError(f'{path}: {m.group(2)} declared twice')
        out[m.group(2)] = h
    return out


def handlers():
    """every declared handler: Java name -> {'kind', 'file', 'tokens', 'effects', 'filters'}"""
    global _cache
    if _cache is None:
        _cache = {}
        for path in sorted(glob.glob(os.path.join(DIR, '**', '*.rs'), recursive=True)):
            if os.path.basename(path) == 'mod.rs':
                continue
            for k, v in _parse(open(path, encoding='utf-8').read(), path).items():
                if k in _cache:
                    raise SyntaxError(f'{k} has handlers in {_cache[k]["file"]} and {v["file"]}')
                _cache[k] = v
    return _cache


def outer_class(source):
    m = re.search(r'public (?:final )?class (\w+)', source or '')
    return m and m.group(1)


def find(source, cls, kind, own_only=False):
    """the handler's name for a class the card file `source` uses, of this kind ('effect': a one-shot or a static
    kind), or None. `own_only`: only one named for this card (`OuterCardClass.Class`)."""
    hs = handlers()
    outer = outer_class(source)
    for name in ([f'{outer}.{cls}'] if outer else []) + ([] if own_only else [cls]):
        h = hs.get(name)
        if h:
            if h['kind'] != kind and not (kind == 'effect' and h['kind'] in STATIC_KINDS):
                raise _unsupported(f'{kind} {cls}: its handler is a {h["kind"]}')
            return name
    return None


def effect(t, cls, args, own_only=False):
    """`{"class": "CardEffect", …}` for a class with an effect handler (`{"class": "CardStatic", …}` for a static
    one), or None. `t` is transpile's Translator. `own_only`: only a handler named for this card's class."""
    name = find(getattr(t, 'source', None), cls, 'effect', own_only)
    if not name:
        return None
    h = handlers()[name]
    out = {'class': 'CardStatic' if h['kind'] in STATIC_KINDS else 'CardEffect', 'handler': name}
    args = [a.strip() for a in args if a.strip()]
    filters = [t.filter(x) for x in h['filters']]
    abilities = [t.ability(x) for x in h['abilities']]
    # constructor arguments the handler asked to have translated (`.args(&[…])`, one kind per argument):
    # a "library_target" (`TargetCardInLibrary`) becomes "min max" in `args` and its filter goes after the
    # declared filters
    effects = [t.effect(x) for x in h['effects']]
    # "duration": an optional leading `Duration` (a constructor without one is `Duration.WhileOnBattlefield`)
    for i, kind in enumerate(h['args']):
        if kind == 'duration' and (i >= len(args) or not args[i].startswith('Duration.')):
            args.insert(i, 'Duration.WhileOnBattlefield')
        # "until_your_next_turn": a class whose constructor sets `Duration.UntilYourNextTurn` itself (resolved, it
        # lasts until your next turn as a `Duration.UntilYourNextTurn` argument does)
        if kind == 'until_your_next_turn' and i >= len(args):
            args.insert(i, 'Duration.UntilYourNextTurn')
    for i, kind in enumerate(h['args']):
        if i >= len(args):
            break
        if kind == 'library_target':
            filt, lo, hi = t.library_target(args[i])
            filters.append(filt)
            args[i] = f'{lo} {hi}'
        elif kind == 'filter':
            # a filter argument: translated, after the declared filters (and earlier filter arguments)
            filters.append(t.filter(args[i]))
        elif kind == 'effect':
            effects.append(t.effect(args[i]))
        elif kind == 'counter':
            # `CounterType.X.createInstance(n)`: "name n" in `args`
            c = t.value('counter', args[i])
            if not c:
                raise _unsupported(f'counter {args[i]}')
            args[i] = f"{c['name']} {c['amount']}"
        elif kind == 'counter_type':
            # `CounterType.M1M1`: the engine's name for the counter ("-1/-1")
            if args[i] not in T.COUNTERS:
                raise _unsupported(f'argument {args[i]}')
            args[i] = T.COUNTERS[args[i]]
        elif kind == 'put_cards':
            # `PutCards.X`: its engine name (`BATTLEFIELD_TAPPED`)
            m = re.fullmatch(r'PutCards\.(\w+)', args[i])
            if not m:
                raise _unsupported(f'argument {args[i]}')
            args[i] = m.group(1)
        elif kind == 'ability':
            abilities.append(t.ability(args[i]))
        elif kind == 'while_on_battlefield':
            # a static class the handler writes only as a static ability's (`Duration.WhileOnBattlefield`)
            if args[i] != 'Duration.WhileOnBattlefield':
                raise _unsupported(f'effect {cls}: duration {args[i]}')
        elif kind.startswith('one_of:'):
            # an argument the handler reads only some values of ("one_of:NinjutsuAbility.class|EquipAbility.class")
            if args[i] not in kind[len('one_of:'):].split('|'):
                raise _unsupported(f'effect {cls}: argument {args[i]}')
        elif kind not in ('text', 'filter', 'effect', 'counter', 'counter_type', 'put_cards', 'ability',
                          'while_on_battlefield', 'duration', 'until_your_next_turn') \
                and not kind.startswith('one_of:'):
            raise _unsupported(f'argument kind {kind}')
    # a static handler for a framework class is written for its constructors with up to as many arguments as it
    # declares kinds for
    if h['kind'] in STATIC_KINDS and len(args) > len(h['args']):
        raise _unsupported(f'effect {cls}: {len(args)} arguments, its handler takes {len(h["args"])}')
    if args:
        out['args'] = args
    if h['tokens']:
        out['tokens'] = [T.token(x) for x in h['tokens']]
    if effects:
        out['effects'] = effects
    if filters:
        out['filters'] = filters
    if abilities:
        if out['class'] != 'CardStatic':
            raise _unsupported(f'effect {cls}: abilities for a one-shot handler')
        out['abilities'] = abilities
    return out


def mana_ability(t, expr, cost, no_tap):
    """`{"class": "CardManaAbility", …}` for a mana ability whose `ManaEffect` (`new XManaEffect(…)`) has a mana
    handler (`Handler::mana`), with the ability's costs; or None"""
    m = re.fullmatch(r'new (\w+)\((.*)\)', expr.strip(), re.S)
    name = m and find(getattr(t, 'source', None), m.group(1), 'mana')
    if not name:
        return None
    h = handlers()[name]
    args = [a.strip() for a in T.split_args(m.group(2)) if a.strip()] if m.group(2).strip() else []
    out = {'class': 'CardManaAbility', 'handler': name, 'cost': cost, 'no_tap': no_tap}
    if args:
        out['args'] = args
    if h['filters']:
        out['filters'] = [t.filter(x) for x in h['filters']]
    return out


def condition(t, expr):
    """`{"c": "card", …}` for `new XCondition()`, `XCondition.instance` or an enum's constant
    (`XCondition.TEN`, the handler `Outer.XCondition.TEN`) with a condition handler, or None"""
    m = re.fullmatch(r'new (\w+)\(\)|(\w+)\.instance|(\w+\.[A-Z][A-Z0-9_]*)', expr.strip())
    name = m and find(getattr(t, 'source', None), m.group(1) or m.group(2) or m.group(3), 'condition')
    # `new XCondition(…)` with arguments: only a handler named for the card (`OuterCardClass.XCondition`, written
    # for the card's arguments)
    if not m and (m := re.fullmatch(r'new (\w+)\(.+\)', expr.strip(), re.S)):
        name = find(getattr(t, 'source', None), m.group(1), 'condition', own_only=True)
    return {'c': 'card', 'handler': name} if name else None


def value(t, expr):
    """`{"v": "card", …}` for `new XValue()` or `XValue.instance` with a value handler, or None; `new XValue(…)` with
    arguments only with a handler named for the card (`OuterCardClass.XValue`, written for the card's arguments)"""
    m = re.fullmatch(r'new (\w+)\(\)|(\w+)\.instance', expr.strip())
    name = m and find(getattr(t, 'source', None), m.group(1) or m.group(2), 'value')
    if not m and (m := re.fullmatch(r'new (\w+)\(.+\)', expr.strip(), re.S)):
        name = find(getattr(t, 'source', None), m.group(1), 'value', own_only=True)
    return {'v': 'card', 'handler': name} if name else None


def predicate(expr):
    """`{"p": "card", …}` for `XPredicate.instance` or `new XPredicate()` with a predicate handler, or None"""
    m = re.fullmatch(r'new (\w+)\(\)|(\w+)\.instance', expr.strip())
    name = m and find(SOURCE, m.group(1) or m.group(2), 'predicate')
    # `new XPredicate(…)` with arguments: only a handler named for the card (written for the card's arguments)
    if not m and (m := re.fullmatch(r'new (\w+)\(.+\)', expr.strip(), re.S)):
        name = find(SOURCE, m.group(1), 'predicate', own_only=True)
    return {'p': 'card', 'handler': name} if name else None


def trigger(t, cls, args):
    """A triggered ability class with a trigger handler: its base trigger, translated, with the handler as its
    `check` and the targets its constructor adds; or None"""
    name = find(getattr(t, 'source', None), cls, 'trigger')
    if not name:
        return None
    base = handlers()[name]['base']
    args = [a.strip() for a in args if a.strip()]
    # a base with `$0`, `$1`…: the constructor's arguments go there (Ob Nixilis, Captive Kingpin's effect)
    holes = sorted({int(x) for x in re.findall(r'\$(\d+)', base)})
    if holes != list(range(len(args))):
        raise _unsupported(f'ability {cls}: {len(args)} constructor arguments, its handler takes {len(holes)}')
    for i, arg in enumerate(args):
        base = base.replace('$%d' % i, arg)
    a = t.ability(base)
    if a.get('class') != 'TriggeredAbility':
        a = T.triggered(a)
    if a.get('class') != 'TriggeredAbility':
        raise _unsupported(f'ability {cls}: its base trigger is a {a.get("class")}')
    # targets its constructor adds (`this.addTarget(…)`), after the base trigger's own
    inner = cls.split('.')[-1]
    body = re.search(r'\n\s*(?:public |protected |private )?' + inner + r'\(\)\s*\{(.*?)\n    \}',
                     getattr(t, 'source', '') or '', re.S)
    for m in re.finditer(r'(?:this\.)?addTarget\((.*?)\);', body.group(1) if body else '', re.S):
        a.setdefault('targets', []).append(t.target(m.group(1)))
    a['java'] = cls
    a['check'] = name
    return a

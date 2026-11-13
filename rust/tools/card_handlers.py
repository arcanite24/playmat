"""Per-card handlers (docs/rust/Engine.md, "Per-card handlers"): the Java classes written by hand in Rust, one file
per card under rust/crates/playmat-engine/src/cards/. transpile.py asks here before it gives up on a class a card
declares for itself, and emits a reference to the handler instead.

A handler file declares its classes as

    pub const HANDLERS: &[Handler] = &[
        Handler::effect("RapidHybridization.RapidHybridizationEffect", apply).tokens(&["FrogLizardToken"]),
    ];

`Handler::effect`, `::condition`, `::value` or `::trigger` (with the framework trigger it extends, in Java, as its
second argument), with the Java class as `OuterCardClass.InnerClass` (or the bare class
name for one in the framework), and optionally `.tokens(&[…])`, `.effects(&[…])`, `.filters(&[…])`: Java for the
transpiler to translate (token classes, effect and filter expressions), handed to the handler in that order; and
`.args(&[…])`, one kind per constructor argument ("text" as is, "library_target": a `TargetCardInLibrary`, as
"min max" with its filter after the declared ones). The
strings are plain or raw Rust string literals.
"""
import glob
import os
import re

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
DIR = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'src', 'cards')

_DECL = re.compile(r'Handler::(effect|condition|value|trigger)\(\s*"([\w.]+)"\s*,')
_FN = re.compile(r'\s*[\w:]+\s*,?\s*\)')
_PART = re.compile(r'\s*\.(tokens|effects|filters|args)\(\s*&\[')
_STR = re.compile(r'\s*(?:"((?:[^"\\]|\\.)*)"|r(#*)"(.*?)"\2)\s*,?', re.S)

_cache = None
# transpile.py, set as it imports this module (for `token`)
T = None


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
             'args': [], 'base': None}
        i = m.end()
        if m.group(1) == 'trigger':
            # `Handler::trigger("X", "<framework trigger in Java>", f)`
            b = _STR.match(text, i)
            if not b or not b.group(0).strip():
                raise SyntaxError(f'{path}: Handler::trigger takes its base trigger as a string literal')
            h['base'] = _unescape(b.group(1)) if b.group(1) is not None else b.group(3)
            i = b.end()
        f = _FN.match(text, i)
        if not f:
            raise SyntaxError(f'{path}: Handler::{m.group(1)}("{m.group(2)}", ...) not understood')
        i = f.end()
        while (p := _PART.match(text, i)):
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


def find(source, cls, kind):
    """the handler's name for a class the card file `source` uses, of this kind, or None"""
    hs = handlers()
    outer = outer_class(source)
    for name in ([f'{outer}.{cls}'] if outer else []) + [cls]:
        h = hs.get(name)
        if h:
            if h['kind'] != kind:
                raise _unsupported(f'{kind} {cls}: its handler is a {h["kind"]}')
            return name
    return None


def effect(t, cls, args):
    """`{"class": "CardEffect", …}` for a class with an effect handler, or None. `t` is transpile's Translator."""
    name = find(getattr(t, 'source', None), cls, 'effect')
    if not name:
        return None
    h = handlers()[name]
    out = {'class': 'CardEffect', 'handler': name}
    args = [a.strip() for a in args if a.strip()]
    filters = [t.filter(x) for x in h['filters']]
    # constructor arguments the handler asked to have translated (`.args(&[…])`, one kind per argument):
    # a "library_target" (`TargetCardInLibrary`) becomes "min max" in `args` and its filter goes after the
    # declared filters
    for i, kind in enumerate(h['args']):
        if kind == 'library_target' and i < len(args):
            filt, lo, hi = t.library_target(args[i])
            filters.append(filt)
            args[i] = f'{lo} {hi}'
        elif kind != 'text':
            raise _unsupported(f'argument kind {kind}')
    if args:
        out['args'] = args
    if h['tokens']:
        out['tokens'] = [T.token(x) for x in h['tokens']]
    if h['effects']:
        out['effects'] = [t.effect(x) for x in h['effects']]
    if filters:
        out['filters'] = filters
    return out


def condition(t, expr):
    """`{"c": "card", …}` for `new XCondition()` or `XCondition.instance` with a condition handler, or None"""
    m = re.fullmatch(r'new (\w+)\(\)|(\w+)\.instance', expr.strip())
    name = m and find(getattr(t, 'source', None), m.group(1) or m.group(2), 'condition')
    return {'c': 'card', 'handler': name} if name else None


def value(t, expr):
    """`{"v": "card", …}` for `new XValue()` or `XValue.instance` with a value handler, or None"""
    m = re.fullmatch(r'new (\w+)\(\)|(\w+)\.instance', expr.strip())
    name = m and find(getattr(t, 'source', None), m.group(1) or m.group(2), 'value')
    return {'v': 'card', 'handler': name} if name else None


def trigger(t, cls, args):
    """A triggered ability class with a trigger handler: its base trigger, translated, with the handler as its
    `check`; or None"""
    name = find(getattr(t, 'source', None), cls, 'trigger')
    if not name:
        return None
    if [a for a in args if a.strip()]:
        raise _unsupported(f'ability {cls}: a trigger handler takes no constructor arguments')
    a = t.ability(handlers()[name]['base'])
    if a.get('class') != 'TriggeredAbility':
        a = T.triggered(a)
    if a.get('class') != 'TriggeredAbility':
        raise _unsupported(f'ability {cls}: its base trigger is a {a.get("class")}')
    a['java'] = cls
    a['check'] = name
    return a

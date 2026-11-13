"""Per-card handlers (docs/rust/Engine.md, "Per-card handlers"): the Java classes written by hand in Rust, one file
per card under rust/crates/playmat-engine/src/cards/. transpile.py asks here before it gives up on a class a card
declares for itself, and emits a reference to the handler instead.

A handler file declares its classes as

    pub const HANDLERS: &[Handler] = &[
        Handler::effect("RapidHybridization.RapidHybridizationEffect", apply).tokens(&["FrogLizardToken"]),
    ];

`Handler::effect`, `::condition` or `::value`, with the Java class as `OuterCardClass.InnerClass` (or the bare class
name for one in the framework), and optionally `.tokens(&[…])`, `.effects(&[…])`, `.filters(&[…])`: Java for the
transpiler to translate (token classes, effect and filter expressions), handed to the handler in that order. The
strings are plain or raw Rust string literals.
"""
import glob
import os
import re

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
DIR = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'src', 'cards')

_DECL = re.compile(r'Handler::(effect|condition|value)\(\s*"([\w.]+)"\s*,\s*[\w:]+\s*,?\s*\)')
_PART = re.compile(r'\s*\.(tokens|effects|filters)\(\s*&\[')
_STR = re.compile(r'\s*(?:"((?:[^"\\]|\\.)*)"|r(#*)"(.*?)"\2)\s*,?', re.S)

_cache = None
# transpile.py, set as it imports this module (for `token`)
T = None


class Unsupported(Exception):
    pass


def _unescape(s):
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), s)


def _parse(text, path):
    out = {}
    for m in _DECL.finditer(text):
        h = {'kind': m.group(1), 'file': os.path.relpath(path, ROOT), 'tokens': [], 'effects': [], 'filters': []}
        i = m.end()
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
                raise Unsupported(f'{kind} {cls}: its handler is a {h["kind"]}')
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
    if args:
        out['args'] = args
    if h['tokens']:
        out['tokens'] = [T.token(x) for x in h['tokens']]
    if h['effects']:
        out['effects'] = [t.effect(x) for x in h['effects']]
    if h['filters']:
        out['filters'] = [t.filter(x) for x in h['filters']]
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

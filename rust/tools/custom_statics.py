"""A card's own static effect classes (docs/rust/Engine.md, "Cards' own effects"): replacement effects
(`checksEventType`, `applies`, `replaceEvent`), continuous effects (`apply` in a layer), cost changes, rule-modifying
and restriction effects, read by `custom_effects.translate` when the class isn't a `OneShotEffect`.

Their methods are short and come in a few shapes: "a triggered ability of a creature you control triggers an
additional time", "if one or more tokens would be created under your control, twice that many are", "creatures you
control get +1/+1". Each shape is one engine effect; the class's `applies` test (who and what it applies to) is read
into that effect's filters:

- the test's statements become one formula (`_formula`): a null check that returns false, `if (…) return …;`, locals
  (`Permanent p = game.getPermanent(event.getSourceId());`, a cast `(NumberOfTriggersEvent) event`);
- each atom says something about a permanent the event names (`StaticTest.atom`: its source, its target, the
  permanents that entered or died, a stream's `anyMatch`), about the event's player ("you"), or is a condition;
- the shape's reader takes the conjuncts it knows and leaves the class unsupported on any other.
"""
import re

from custom_effects import Unsupported, TYPE_CHECKS, parse, method_body, _split, _balanced, _ends_with_return

# the transpiler module (custom_effects sets it)
T = None

KINDS = ('ReplacementEffectImpl', 'ContinuousEffectImpl', 'CostModificationEffectImpl',
         'ContinuousRuleModifyingEffectImpl', 'RestrictionEffect', 'PreventionEffectImpl')


def translate(t, cls, extends, body, transpiler):
    """a card's own static effect class -> an engine effect, or Unsupported"""
    global T
    T = transpiler
    # a static ability's effect lasts while its source is on the battlefield; one a resolving ability creates (for a
    # turn) isn't one of these
    durations = set(re.findall(r'\bsuper\(\s*Duration\.(\w+)', body))
    if durations != {'WhileOnBattlefield'}:
        raise Unsupported('duration ' + ','.join(sorted(durations)))
    if extends == 'ReplacementEffectImpl':
        events = set(re.findall(r'EventType\.(\w+)', method_body(body, 'checksEventType')[1] or ''))
        if events == {'NUMBER_OF_TRIGGERS'}:
            return _additional_triggers(t, body)
        if events == {'ADD_COUNTERS'}:
            return _modify_counters(t, body)
        if events == {'CREATE_TOKEN'}:
            return _tokens(t, body)
        if events == {'ENTERS_THE_BATTLEFIELD'}:
            return _enters_with(t, body)
        if events in ({'DAMAGE_PLAYER', 'DAMAGE_PERMANENT'}, {'DAMAGE_PLAYER'}):
            return _damage(t, body, events == {'DAMAGE_PLAYER'})
        raise Unsupported('replacement of ' + ','.join(sorted(events)))
    raise Unsupported('extends ' + extends)


# ---------------------------------------------------------------------------------------------------------------------
# formulas

def _formula(nodes, env):
    """a boolean method's statements -> when it returns true: ('t',) ('f',) ('atom', text) ('not', f) ('and', a, b)
    ('or', a, b). Locals (`Permanent p = …;`) go in `env`, by name, and are written out in the atoms."""
    if not nodes:
        return ('f',)
    n, rest = nodes[0], nodes[1:]
    if n[0] == 'block':
        return _formula(n[1] + rest, env)
    if n[0] == 'loop':
        raise Unsupported('loop in a test')
    if n[0] == 'if':
        c = _expr(n[1], env)
        then = n[2] if _ends_with_return(n[2]) else n[2] + rest
        other = (n[3] or []) if _ends_with_return(n[3] or []) else (n[3] or []) + rest
        return _or(_and(c, _formula(then, env)), _and(_not(c), _formula(other, env)))
    st = n[1]
    m = re.fullmatch(r'return (.*)', st)
    if m:
        return _expr(m.group(1), env)
    m = re.fullmatch(r'(?:final )?([\w.<>]+) (\w+)=(.*)', st)
    if m:
        env = dict(env)
        rhs = m.group(3)
        c = re.fullmatch(r'\((\w+)\)\s*(\w+)', rhs)
        env[m.group(2)] = env.get(c.group(2), c.group(2)) if c else _resolve(rhs, env)
        return _formula(rest, env)
    # a local set again on this path (`if (p == null) { p = game.getPermanentEntering(…); }`)
    m = re.fullmatch(r'(\w+)=(.*)', st)
    if m and m.group(1) in env:
        env = dict(env)
        env[m.group(1)] = _resolve(m.group(2), env)
        return _formula(rest, env)
    raise Unsupported('test statement ' + st[:60])


def _and(a, b):
    if a == ('f',) or b == ('f',):
        return ('f',)
    if a == ('t',):
        return b
    if b == ('t',):
        return a
    return ('and', a, b)


def _or(a, b):
    if a == b:
        return a
    if a == ('t',) or b == ('t',):
        return ('t',)
    if a == ('f',):
        return b
    if b == ('f',):
        return a
    return ('or', a, b)


def _not(a):
    if a == ('t',):
        return ('f',)
    if a == ('f',):
        return ('t',)
    if a[0] == 'not':
        return a[1]
    return ('not', a)


def _unparen(e):
    e = e.strip()
    while re.fullmatch(r'\((.*)\)', e, re.S) and _balanced(e[1:-1]):
        e = e[1:-1].strip()
    return e


def _expr(e, env):
    e = _unparen(e)
    if e == 'true':
        return ('t',)
    if e == 'false':
        return ('f',)
    for op, f in (('||', _or), ('&&', _and)):
        parts = _split(e, op)
        if len(parts) > 1:
            out = _expr(parts[0], env)
            for p in parts[1:]:
                out = f(out, _expr(p, env))
            return out
    if e.startswith('!') and not e.startswith('!='):
        return _not(_expr(e[1:], env))
    return ('atom', _resolve(e, env))


def _resolve(e, env):
    """locals written out: `p.isCreature(game)` -> `game.getPermanent(event.getSourceId()).isCreature(game)`"""
    if not env:
        return e
    return re.sub(r'(?<![\w.])(\w+)(?![\w(])', lambda m: env.get(m.group(1), m.group(1)), e)


# ---------------------------------------------------------------------------------------------------------------------
# what a test's atoms say

# the permanents an event names, in a replacement effect's test
SUBJECTS = [
    (r'game\.(?:getPermanentOrLKIBattlefield|getPermanent)\(event\.getSourceId\(\)\)', 'event_source'),
    (r'game\.(?:getPermanentOrLKIBattlefield|getPermanent)\(\(?(?:\(NumberOfTriggersEvent\))?event\)?\.getSourceId\(\)\)', 'event_source'),
    (r'game\.(?:getPermanentOrLKIBattlefield|getPermanent|getObject)\(event\.getSourceId\(\)\)', 'event_source'),
    (r'game\.(?:getPermanentOrLKIBattlefield|getPermanent)\(event\.getTargetId\(\)\)', 'event_target'),
    (r'game\.getPermanentEntering\(event\.getTargetId\(\)\)', 'event_target'),
    (r'Optional\s*\.ofNullable\(event\)\s*\.map\(GameEvent::getTargetId\)\s*\.map\(game::getPermanent\)\s*'
     r'\.orElse(?:Get)?\((?:\(\)\s*->\s*)?game\.getPermanentEntering\(event\.getTargetId\(\)\)\)', 'event_target'),
    (r'\(?(?:\(EntersTheBattlefieldEvent\))?event\)?\.getTarget\(\)', 'event_target'),
    (r'game\.(?:getPermanentOrLKIBattlefield|getPermanent)\(source\.getSourceId\(\)\)', 'source'),
    (r'source\.getSourcePermanent(?:IfItStillExists|OrLKI)\(game\)', 'source'),
]

CMP_OPS = {'<': 'FEWER_THAN', '>': 'MORE_THAN', '==': 'EQUAL_TO', '<=': 'OR_LESS', '>=': 'OR_GREATER'}
COLOR_CHECKS = {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}
YOU_ARE_THE_EVENTS_PLAYER = re.compile(
    r'source\.isControlledBy\(event\.getPlayerId\(\)\)|event\.getPlayerId\(\)\.equals\(source\.getControllerId\(\)\)|'
    r'source\.getControllerId\(\)\.equals\(event\.getPlayerId\(\)\)|event\.getPlayerId\(\)==source\.getControllerId\(\)|'
    r'source\.isControlledBy\(\(\(NumberOfTriggersEvent\)event\)\.getPlayerId\(\)\)')


class StaticTest:
    """atoms of an `applies` test -> ('pred', subject, pred) for a permanent's predicate, ('you',) for "the event's
    player is you", ('cond', c) for a condition, ('any', subject, filter) for "one of the permanents that entered (or
    died) is of the filter", ('t',)/('f',) for what always (never) holds"""

    def __init__(self, t, filters, lambdas=None):
        self.t = t
        self.filters = filters
        self.lambdas = lambdas or {}

    def subject(self, expr):
        expr = _unparen(expr)
        if expr in self.lambdas:
            return self.lambdas[expr]
        for pat, role in SUBJECTS:
            if re.fullmatch(pat, expr):
                return role
        return None

    def atom(self, a):
        # comparisons with a number, without spaces (`canonical` keeps them around `<` and `>`)
        a = re.sub(r'(?<!-)\s*([<>])\s*(?=-?\d)', r'\1', a.strip())
        if re.fullmatch(r'event\.getType\(\)==(?:GameEvent\.)?EventType\.\w+|\(?event\)? instanceof \w+', a):
            return ('t',)
        m = re.fullmatch(r'(.*)(!=|==)null', a)
        if m and _balanced(m.group(1)) and (self.subject(m.group(1)) or re.fullmatch(r'game\.getPlayer\(.*\)', m.group(1))):
            return ('t',) if m.group(2) == '!=' else ('f',)
        if re.fullmatch(r'\(?(?:\(NumberOfTriggersEvent\))?\w+\)?\.getSourceEvent\(\)!=null', a):
            return ('t',)
        m = re.fullmatch(r'(Equipped|Enchanted|Attacking|Blocking|Tapped|Untapped|Modified|Token)Predicate\.(?:instance|TRUE|TAPPED|UNTAPPED)\.apply\((.*),game\)', a)
        if m and (s := self.subject(m.group(2))):
            return ('pred', s, {'Equipped': {'p': 'equipped'}, 'Enchanted': {'p': 'enchanted'}, 'Attacking': {'p': 'attacking'},
                                'Blocking': {'p': 'blocking'}, 'Tapped': {'p': 'tapped', 'v': True},
                                'Untapped': {'p': 'tapped', 'v': False}, 'Modified': {'p': 'modified'},
                                'Token': {'p': 'token', 'v': True}}[m.group(1)])
        if YOU_ARE_THE_EVENTS_PLAYER.fullmatch(a):
            return ('you',)
        # `ADD_COUNTERS`: the counters' kind, how many, put by an effect
        m = re.fullmatch(r'event\.getData\(\)\.equals\((CounterType\.\w+)\.getName\(\)\)|(CounterType\.\w+)\.getName\(\)\.equals\(event\.getData\(\)\)', a)
        if m and (m.group(1) or m.group(2)) in T.COUNTERS:
            return ('counter', T.COUNTERS[m.group(1) or m.group(2)])
        m = re.fullmatch(r'event\.getAmount\(\)(<=|>=|<|>|==|!=)(\d+)', a)
        if m:
            # the engine asks only with one or more: "> 0" always holds, "< 1" never does
            op, n = m.group(1), int(m.group(2))
            always = {'>': n < 1, '>=': n <= 1}.get(op)
            never = {'<': n <= 1, '<=': n < 1}.get(op)
            if always:
                return ('t',)
            if never:
                return ('f',)
            raise Unsupported('test ' + a)
        if a == 'event.getFlag()':
            return ('flag',)
        # damage events: whose source, combat or not, to an opponent
        if re.fullmatch(r'game\.getControllerId\(event\.getSourceId\(\)\)\.equals\(source\.getControllerId\(\)\)|'
                        r'source\.isControlledBy\(game\.getControllerId\(event\.getSourceId\(\)\)\)', a):
            return ('pred', 'event_source', {'p': 'controller', 'who': 'you'})
        if re.fullmatch(r'source\.getSourceId\(\)\.equals\(event\.getSourceId\(\)\)|event\.getSourceId\(\)\.equals\(source\.getSourceId\(\)\)', a):
            return ('this_source',)
        if re.fullmatch(r'\(\(DamageEvent\)event\)\.isCombatDamage\(\)', a):
            return ('combat',)
        if re.fullmatch(r'game\.getPlayer\(source\.getControllerId\(\)\)\.hasOpponent\((?:event\.getTargetId\(\)|game\.getControllerId\(event\.getTargetId\(\)\)),game\)', a):
            return ('to_opponent',)
        m = re.fullmatch(r'event\.getSourceId\(\)\.equals\((.*)\.getAttachedTo\(\)\)', a)
        if m and self.subject(m.group(1)) == 'source':
            return ('attached_source',)
        m = re.fullmatch(r'(.*)\.getAttachedTo\(\)!=null', a)
        if m and self.subject(m.group(1)) == 'source':
            return ('t',)
        m = re.fullmatch(r'(.*?)\.getTurnsOnBattlefield\(\)==0', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'entered_this_turn'})
        # `Optional.ofNullable(x).map(p -> …).orElse(false)`: what the lambda says about x
        m = re.fullmatch(r'Optional\s*\.ofNullable\((.*?)\)\s*\.map\((\w+)\s*->\s*(.*)\)\s*\.orElse\(false\)', a, re.S)
        if m and _balanced(m.group(1)) and (s := self.subject(m.group(1))):
            inner = StaticTest(self.t, self.filters, dict(self.lambdas, **{m.group(2): s}))
            return inner.read(_expr(m.group(3), {}))
        m = re.fullmatch(r'game\.getPlayer\(event\.getTargetId\(\)\)(==|!=)null', a)
        if m:
            # counters on a player: the engine has none of these replacements for players yet
            return ('t',) if m.group(1) == '==' else ('f',)
        m = re.fullmatch(r'(.*?)\.(is\w+)\(game\)', a)
        if m and m.group(2) in TYPE_CHECKS and (s := self.subject(m.group(1))):
            preds = TYPE_CHECKS[m.group(2)]
            return ('pred', s, preds[0] if len(preds) == 1 else {'p': 'and', 'xs': preds})
        m = re.fullmatch(r'(.*?)\.hasSubtype\(SubType\.(\w+),game\)', a)
        if m and (s := self.subject(m.group(1))) and m.group(2) in T.subtype_names():
            return ('pred', s, {'p': 'subtype', 's': T.subtype_names()[m.group(2)]})
        m = re.fullmatch(r'(.*?)\.hasSubtype\(ChooseCreatureTypeEffect\.getChosenCreatureType\(source\.getSourceId\(\),game\),game\)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'chosen_subtype', 'v': True})
        m = re.fullmatch(r'(.*?)\.(?:isControlledBy\(source\.getControllerId\(\)\)|getControllerId\(\)\.equals\(source\.getControllerId\(\)\))', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'controller', 'who': 'you'})
        m = re.fullmatch(r'game\.getOpponents\(source\.getControllerId\(\)\)\.contains\((.*?)\.getControllerId\(\)\)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'controller', 'who': 'opponent'})
        m = re.fullmatch(r'(.*?) instanceof PermanentToken', a) or re.fullmatch(r'(.*?)\.isToken\(\)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'token', 'v': True})
        m = re.fullmatch(r'(.*?)\.get(Power|Toughness)\(\)\.getValue\(\)(<=|>=|<|>|==)(\d+)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': m.group(2).lower(), 'cmp': CMP_OPS[m.group(3)], 'n': int(m.group(4))})
        m = re.fullmatch(r'(.*?)\.getManaValue\(\)(<=|>=|<|>|==)(\d+)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'mv', 'cmp': CMP_OPS[m.group(2)], 'n': int(m.group(3))})
        m = re.fullmatch(r'(.*?)\.getId\(\)\.equals\(source\.getSourceId\(\)\)', a) \
            or re.fullmatch(r'source\.getSourceId\(\)\.equals\((.*?)\.getId\(\)\)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'not', 'x': {'p': 'another'}})
        m = re.fullmatch(r'(\w+)\.match\((.*?),(?:source\.getControllerId\(\),source,)?game\)', a)
        if m and m.group(1) in self.filters and (s := self.subject(m.group(2))):
            f = self.filters[m.group(1)]
            if isinstance(f, Exception):
                raise Unsupported('filter ' + m.group(1))
            return ('pred', s, f[0] if len(f) == 1 else {'p': 'and', 'xs': f})
        m = re.fullmatch(r'(.*?)\.getColor\(game\)\.is(White|Blue|Black|Red|Green)\(\)', a)
        if m and (s := self.subject(m.group(1))):
            return ('pred', s, {'p': 'color', 'c': COLOR_CHECKS[m.group(2)]})
        m = re.fullmatch(r'(\w+Condition\.\w+)\.apply\(game,source\)', a)
        if m:
            try:
                return ('cond', self.t.condition(m.group(1)))
            except Exception:  # noqa: BLE001: the translator's own Unsupported
                raise Unsupported('condition ' + m.group(1))
        # one of the permanents whose entering (or dying) caused it (a stream's anyMatch)
        ev = r'\(?(?:\(NumberOfTriggersEvent\))?event\)?'
        m = re.fullmatch(ev + r'\.get(Entered|Died)Permanents\(game\)\.stream\(\)\.(anyMatch|noneMatch)\((\w+) -> (.*)\)', a)
        if m:
            role = m.group(1).lower()
            inner = StaticTest(self.t, self.filters, dict(self.lambdas, **{m.group(3): role}))
            out = ('any', role, _conj_preds(inner.read(_expr(m.group(4), {})), role))
            return out if m.group(2) == 'anyMatch' else ('not', out)
        m = re.fullmatch(ev + r'\.get(Entered|Died)Permanents\(game\)\.isEmpty\(\)', a)
        if m:
            return ('not', ('any', m.group(1).lower(), []))
        raise Unsupported('test ' + a[:90])

    def read(self, f):
        """a formula -> its atoms' meanings, the same shape"""
        if f[0] in ('t', 'f'):
            return f
        if f[0] == 'atom':
            return self.atom(f[1])
        if f[0] == 'not':
            x = self.read(f[1])
            return _not(x)
        return (_and if f[0] == 'and' else _or)(self.read(f[1]), self.read(f[2]))


def _pred_of(r, role):
    """a read formula about `role` -> one predicate"""
    if r[0] == 'pred' and r[1] == role:
        return r[2]
    if r[0] == 'not':
        return {'p': 'not', 'x': _pred_of(r[1], role)}
    if r[0] in ('and', 'or'):
        return {'p': r[0], 'xs': [_pred_of(r[1], role), _pred_of(r[2], role)]}
    raise Unsupported('test about something else: ' + repr(r)[:60])


def _conj_preds(r, role):
    if r == ('t',):
        return []
    if r[0] == 'and':
        return _conj_preds(r[1], role) + _conj_preds(r[2], role)
    return [_pred_of(r, role)]


def _conjuncts(r):
    """a read formula -> its conjuncts"""
    if r == ('t',):
        return []
    if r == ('f',):
        raise Unsupported('never applies')
    if r[0] == 'and':
        return _conjuncts(r[1]) + _conjuncts(r[2])
    return [r]


def _subjects(r):
    """the subjects a read formula is about"""
    if r[0] == 'pred':
        return {r[1]}
    if r[0] in ('not',):
        return _subjects(r[1])
    if r[0] in ('and', 'or'):
        return _subjects(r[1]) | _subjects(r[2])
    return {None}


def class_filters(t, body):
    """the static filter fields of an effect class (`static { filter.add(…); }`), by name"""
    return T.declared_filters(body, t.subtypes)


def _applies(t, body, filters, params=r'GameEvent event,Ability source,Game game'):
    p, applies = method_body(body, 'applies')
    if applies is None:
        raise Unsupported('no applies')
    if re.sub(r'\s+|final ', '', p or '') != params.replace(' ', ''):
        raise Unsupported('applies(' + (p or '') + ')')
    test = StaticTest(t, filters)
    return test.read(_formula(parse(applies), {}))


# ---------------------------------------------------------------------------------------------------------------------
# shapes

def _additional_triggers(t, body):
    """"a triggered ability … triggers an additional time": `event.setAmount(event.getAmount() + 1)`"""
    _, replace = method_body(body, 'replaceEvent')
    if not re.fullmatch(r'event\.setAmount\((?:event\.getAmount\(\)\+1|CardUtil\.overflowInc\(event\.getAmount\(\),1\))\);returnfalse;',
                        re.sub(r'\s+', '', replace or '')):
        raise Unsupported('replaceEvent ' + re.sub(r'\s+', ' ', replace or '')[:60])
    r = _applies(t, body, class_filters(t, body))
    out = {'class': 'AdditionalTriggersEffect'}
    you = False
    for c in _conjuncts(r):
        if c == ('you',):
            you = True
        elif c[0] == 'any' and c[1] in ('entered', 'died'):
            out.setdefault(c[1], []).extend(c[2])
        elif c[0] == 'cond':
            out['condition'] = c[1] if 'condition' not in out else {'c': 'and', 'xs': [out['condition'], c[1]]}
        elif _subjects(c) == {'event_source'}:
            p = _pred_of(c, 'event_source')
            out.setdefault('source', []).extend(p['xs'] if p['p'] == 'and' else [p])
        else:
            raise Unsupported('test part ' + repr(c)[:80])
    # the trigger's controller is the event's player: a source "you control" says the same
    src = out.get('source', [])
    if {'p': 'controller', 'who': 'you'} in src:
        src.remove({'p': 'controller', 'who': 'you'})
        you = True
    if not you:
        raise Unsupported("a trigger of any player's")
    if 'source' in out and not out['source']:
        del out['source']
    # `game.getPermanent(event.getSourceId()) != null`: its source is on the battlefield now (not one that left)
    _, applies = method_body(body, 'applies')
    if re.search(r'game\.getPermanent\(\(?(?:\(NumberOfTriggersEvent\))?\w+\)?\.getSourceId\(\)\)', re.sub(r'\s+', '', applies)):
        out['on_battlefield'] = True
    return out


def _replace_body(body):
    _, replace = method_body(body, 'replaceEvent')
    return re.sub(r'\s+', '', replace or '')


def _modify_counters(t, body):
    """"if one or more counters would be put on …, twice that many (that many plus one) are put on it instead":
    `event.setAmountForCounters(…, true)`"""
    r = _replace_body(body)
    m = re.fullmatch(r'event\.setAmountForCounters\((CardUtil\.overflowMultiply\(event\.getAmount\(\),(\d+)\)|'
                     r'CardUtil\.overflowInc\(event\.getAmount\(\),(\d+)\)|event\.getAmount\(\)([+-]\d+)),true\);returnfalse;', r)
    if not m:
        raise Unsupported('replaceEvent ' + r[:70])
    out = {'class': 'ModifyCountersAddedEffect', 'filter': []}
    if m.group(2):
        out['times'] = int(m.group(2))
    else:
        out['plus'] = int(m.group(3) or m.group(4))
    for c in _conjuncts(_applies(t, body, class_filters(t, body))):
        if c == ('you',):
            out['by_you'] = True
        elif c == ('flag',):
            out['effects_only'] = True
        elif c[0] == 'counter':
            if out.get('counter', c[1]) != c[1]:
                raise Unsupported('two counter kinds')
            out['counter'] = c[1]
        elif _subjects(c) == {'event_target'}:
            p = _pred_of(c, 'event_target')
            out['filter'].extend(p['xs'] if p['p'] == 'and' else [p])
        else:
            raise Unsupported('test part ' + repr(c)[:80])
    return out


def _tokens(t, body):
    """"if an effect would create one or more tokens under your control, it creates twice that many instead"
    (`multiplyTokens(2)`): the engine's token doubler"""
    r = _replace_body(body)
    if not re.fullmatch(r'(?:if\(eventinstanceofCreateTokenEvent\)\{)?\(\(CreateTokenEvent\)event\)\.multiplyTokens\(2\);\}?returnfalse;', r):
        raise Unsupported('replaceEvent ' + r[:70])
    if _conjuncts(_applies(t, body, class_filters(t, body))) != [('you',)]:
        raise Unsupported('tokens of a filter')
    return {'class': 'CreateTwiceThatManyTokensEffect'}


DAMAGE_AMOUNT = re.compile(r'event\.setAmount\((?:CardUtil\.overflowMultiply\(event\.getAmount\(\),(\d+)\)|'
                           r'CardUtil\.overflowInc\(event\.getAmount\(\),(\d+|event\.getAmount\(\))\))\);returnfalse;')


def _damage(t, body, players_only):
    """damage replacements: "if a source you control would deal damage …, it deals double (that much plus 1) instead"."""
    r = _replace_body(body)
    m = DAMAGE_AMOUNT.fullmatch(r)
    if not m:
        raise Unsupported('replaceEvent ' + r[:70])
    out = {'class': 'ModifyDamageEffect'}
    if m.group(1):
        out['times'] = int(m.group(1))
    elif m.group(2) == 'event.getAmount()':
        out['times'] = 2
    else:
        out['plus'] = int(m.group(2))
    if players_only:
        out['players_only'] = True
    for c in _conjuncts(_applies(t, body, class_filters(t, body))):
        if c in (('this_source',), ('attached_source',), ('to_opponent',)):
            out[c[0]] = True
        elif c == ('combat',):
            out['combat'] = True
        elif c == ('not', ('combat',)):
            out['combat'] = False
        elif c[0] == 'cond':
            out['condition'] = c[1] if 'condition' not in out else {'c': 'and', 'xs': [out['condition'], c[1]]}
        elif _subjects(c) == {'event_source'}:
            p = _pred_of(c, 'event_source')
            out.setdefault('source', []).extend(p['xs'] if p['p'] == 'and' else [p])
        else:
            raise Unsupported('test part ' + repr(c)[:80])
    return out


ENTERING = r'Permanent(\w+)=\(\(EntersTheBattlefieldEvent\)event\)\.getTarget\(\);'
RIOT = re.compile(ENTERING + r'Playerplayer=game\.getPlayer\(source\.getControllerId\(\)\);'
                  r'if\(\1==null\|\|player==null\)\{returnfalse;\}'
                  r'if\(player\.chooseUse\(outcome,"[^"]*"\+\1\.getLogName\(\)\+"[^"]*",null,"\+1/\+1counter","Haste",source,game\)\)\{'
                  r'\1\.addCounters\(CounterType\.P1P1\.createInstance\(\),source\.getControllerId\(\),source,game,event\.getAppliedEffects\(\)\);'
                  r'\}else\{ContinuousEffecteffect=newGainAbilityTargetEffect\(HasteAbility\.getInstance\(\),Duration\.Custom\);'
                  r'effect\.setTargetPointer\(newFixedTarget\(\1\.getId\(\),\1\.getZoneChangeCounter\(game\)\+1\)\);'
                  r'game\.addEffect\(effect,source\);\}returnfalse;')
ENTERS_WITH = re.compile(ENTERING + r'if\(\1!=null\)\{\1\.addCounters\((CounterType\.\w+)\.createInstance\((\d*)\),'
                         r'source\.getControllerId\(\),source,game,event\.getAppliedEffects\(\)\);\}returnfalse;')


def _enters_with(t, body):
    """"each other creature you control of the chosen type enters with an additional +1/+1 counter" (Metallic Mimic),
    "nontoken creatures you control have riot" (Rhythm of the Wild)"""
    r = _replace_body(body)
    out = {'class': 'EntersWithCountersAllEffect', 'filter': []}
    if RIOT.fullmatch(r):
        out['riot'] = True
    elif (m := ENTERS_WITH.fullmatch(r)) and m.group(2) in T.COUNTERS:
        out['counter'] = {'name': T.COUNTERS[m.group(2)], 'amount': int(m.group(3) or 1)}
    else:
        raise Unsupported('replaceEvent ' + r[:70])
    for c in _conjuncts(_applies(t, body, class_filters(t, body))):
        if _subjects(c) == {'event_target'}:
            p = _pred_of(c, 'event_target')
            out['filter'].extend(p['xs'] if p['p'] == 'and' else [p])
        else:
            raise Unsupported('test part ' + repr(c)[:80])
    return out

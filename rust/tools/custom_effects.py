"""A card's own one-shot effect classes (`class XEffect extends OneShotEffect` in the card's file), read from their
`apply` bodies (docs/rust/Engine.md, "Cards' own effects").

Most of these bodies are a few calls the engine already has an effect for, with null checks around them: "destroy
it, then its controller investigates", "put a +1/+1 counter on this, then create X Goblins where X is its power". Each
statement of the body becomes one engine effect, in order, inside an `Effects` sequence:

- `game.getPlayer(…)`, `game.getPermanent(…)`, `game.getCard(…)` and `source.getSourcePermanent…(game)` locals name
  the controller, the target, or the source; a null check on one stops the sequence there (`require`).
- `int x = …;` works the number out there; what follows is a nested sequence that sees it as its X.
- `new XEffect(…).apply(game, source)`, `game.addEffect(new XEffect(…), source)` and their locals are the effect
  itself (`.setTargetPointer(new FixedTarget(…))` points it at the target's controller, the controller, or the
  source: `to`).
- Calls on those locals (`drawCards`, `gainLife`, `loseLife`, `damage`, `destroy`, `addCounters`, `sacrifice`,
  `moveCards`, …) are the framework effects that make them.

- `Cards` locals are piles (`bind`: the top N cards of a library, what a player mills, a hand, the targets, an empty
  pile): `cards.add`/`remove`/`addAll`/`removeAll` change them, `moveCards(cards, …)`,
  `putCardsOnBottomOfLibrary(cards, …)` and the reveal and look calls act on all of them, and `cards.size()`,
  `cards.count(filter, game)` and `cards.isEmpty()` are numbers and tests.
- A `TargetCard…` local is a choice: `player.choose(outcome, cards, target, source, game)` (or `chooseTarget`, or
  with a hand or graveyard target and no pile) binds what the player chose, a pile itself;
  `game.getCard(target.getFirstTarget())` is its first card.

Anything else in the body (a choice the engine doesn't have, a condition it can't say) leaves the card unsupported,
so a card is translated only when every statement is.
"""
import os
import re

# why each card-specific class wasn't translated, for the report (PLAYMAT_APPLY_LOG=<file>)
LOG = {}


class Unsupported(Exception):
    pass


# the transpiler module (it sets this: run as a script, it isn't importable as `transpile`)
T = None


def _skip_string(text, i):
    q = text[i]
    i += 1
    while i < len(text) and text[i] != q:
        if text[i] == '\\':
            i += 1
        i += 1
    return i + 1


def _matching(text, i, open_, close):
    """the index after the bracket that closes the one at `i`"""
    depth = 0
    while i < len(text):
        c = text[i]
        if c in '"\'':
            i = _skip_string(text, i)
            continue
        if c == open_:
            depth += 1
        elif c == close:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise Unsupported('unbalanced ' + open_)


def _ws(text, i):
    while i < len(text) and text[i].isspace():
        i += 1
    return i


def parse(text):
    """Java statements -> nodes: ('stmt', text), ('if', cond, then, else), ('loop', header, body); blocks are lists"""
    text = re.sub(r'//[^\n]*', '', text)
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    out, i = [], 0
    while True:
        i = _ws(text, i)
        if i >= len(text):
            return out
        node, i = _statement(text, i)
        if node is not None:
            out.append(node)


def _statement(text, i):
    canonical = T.canonical
    i = _ws(text, i)
    if text[i] == '{':
        j = _matching(text, i, '{', '}')
        return ('block', parse(text[i + 1:j - 1])), j
    if text[i] == ';':
        return None, i + 1
    m = re.match(r'(if|for|while|switch|try|do|synchronized)\b', text[i:])
    if m:
        kw = m.group(1)
        if kw == 'switch':
            return _switch(text, i + len(kw))
        if kw in ('try', 'do', 'synchronized'):
            raise Unsupported(kw)
        k = _ws(text, i + len(kw))
        if text[k] != '(':
            raise Unsupported(kw)
        j = _matching(text, k, '(', ')')
        cond = _norm(canonical(text[k + 1:j - 1]))
        then, j = _statement(text, j)
        then = _block(then)
        if kw != 'if':
            return ('loop', kw + '(' + cond + ')', then), j
        k = _ws(text, j)
        other = []
        if re.match(r'else\b', text[k:]):
            other, j = _statement(text, k + 4)
            other = _block(other)
        return ('if', cond, then, other), j
    # a simple statement, to its `;` (lambdas' braces and strings inside count as nested)
    j, depth = i, 0
    while j < len(text):
        c = text[j]
        if c in '"\'':
            j = _skip_string(text, j)
            continue
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ';' and depth == 0:
            return ('stmt', _norm(canonical(text[i:j]))), j + 1
        j += 1
    return ('stmt', canonical(text[i:])), len(text)


def _switch(text, i):
    """`switch (x) { case 1: …; break; case 2: case 3: …; break; default: … }` -> ifs on `x==1`, `x==2||x==3`, else
    the default (each case ends with a break or a return: no fall-through into a case with statements)"""
    canonical = T.canonical
    k = _ws(text, i)
    if text[k] != '(':
        raise Unsupported('switch')
    j = _matching(text, k, '(', ')')
    subject = _norm(canonical(text[k + 1:j - 1]))
    k = _ws(text, j)
    if text[k] != '{':
        raise Unsupported('switch')
    end = _matching(text, k, '{', '}')
    body = text[k + 1:end - 1]
    # the labels at the switch's own level
    labels, depth, p = [], 0, 0
    while p < len(body):
        c = body[p]
        if c in '"\'':
            p = _skip_string(body, p)
            continue
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif depth == 0 and (m := re.match(r'(case\s+([^:]+?)|default)\s*:(?!:)', body[p:])) and (p == 0 or not (body[p - 1].isalnum() or body[p - 1] == '_')):
            labels.append((p, p + m.end(), m.group(2)))
            p += m.end()
            continue
        p += 1
    if not labels or body[:labels[0][0]].strip():
        raise Unsupported('switch')
    cases, pending = [], []
    for n, (start, after, value) in enumerate(labels):
        stop = labels[n + 1][0] if n + 1 < len(labels) else len(body)
        stmts = parse(body[after:stop])
        pending.append(value)
        if not stmts:
            continue
        if stmts[-1] == ('stmt', 'break'):
            stmts = stmts[:-1]
        elif not _ends_with_return(stmts) and n + 1 < len(labels):
            raise Unsupported('switch fall-through')
        if any(s == ('stmt', 'break') for s in _flat(stmts)):
            raise Unsupported('switch break')
        cases.append((pending, stmts))
        pending = []
    default = [s for vs, s in cases if None in vs]
    chain = default[0] if default else []
    for values, stmts in reversed([c for c in cases if None not in c[0]]):
        cond = '||'.join('%s==%s' % (subject, _norm(canonical(v))) for v in values)
        chain = [('if', cond, stmts, chain)]
    return ('block', chain), end


def _norm(text):
    """no spaces around comparisons and logic operators (outside strings)"""
    parts = re.split(r'("(?:[^"\\]|\\.)*")', text)
    return ''.join(p if i % 2 else re.sub(r'\s*(!=|==|&&|\|\||<=|>=)\s*', r'\1', p) for i, p in enumerate(parts))


def _block(node):
    if node is None:
        return []
    return node[1] if node[0] == 'block' else [node]


def class_body(source, cls):
    """(the `extends` class, the class body) of a class declared in a card's file, or None"""
    m = re.search(r'\bclass\s+' + cls + r'\b(?:<[^>]*>)?\s+extends\s+([\w.]+)(?:<[^>{]*>)?[^{]*\{', source)
    if not m:
        return None
    j = _matching(source, m.end() - 1, '{', '}')
    return m.group(1), source[m.end():j - 1]


def declares(source, cls):
    """the card's file declares the class"""
    return re.search(r'\bclass\s+' + cls + r'\b', source) is not None


def method_body(body, name):
    m = re.search(r'(?:public|protected)\s+[\w<>\[\]]+\s+' + name + r'\s*\(([^)]*)\)\s*\{', body)
    if not m:
        return None, None
    j = _matching(body, m.end() - 1, '{', '}')
    return m.group(1), body[m.end():j - 1]


# the expressions that name the ability's first target
FIRST_TARGET = r'(?:(?:this\.)?getTargetPointer\(\)\.getFirst\(game,source\)|source\.getFirstTarget\(\)|' \
               r'source\.getTargets\(\)\.getFirstTarget\(\)|source\.getTargets\(\)\.get\(0\)\.getFirstTarget\(\)|' \
               r'source\.getTargets\(\)\.getByTag\(1\)\.getFirstTarget\(\))'
# the second target
SECOND_TARGET = r'(?:source\.getTargets\(\)\.get\(1\)\.getFirstTarget\(\)|source\.getTargets\(\)\.getByTag\(2\)\.getFirstTarget\(\))'
CONTROLLER_ID = r'(?:source\.getControllerId\(\))'
SOURCE_ID = r'(?:source\.getSourceId\(\))'

# `Kind name = <expr>`: what the local names, as (kind, slot). Kinds: 'you' (the controller), 'player', 'perm' (a
# permanent; 'perm_lki' one that may have left), 'card', 'source', 'source_lki', 'source_object'.
REFS = [
    (r'(?:Player|TargetPlayer)', r'game\.getPlayer\(' + CONTROLLER_ID + r'\)', ('you', 'you')),
    (r'Player', r'game\.getPlayer\(' + FIRST_TARGET + r'\)', ('player', 'target')),
    (r'Permanent', r'game\.getPermanent\(' + FIRST_TARGET + r'\)', ('perm', 'target')),
    (r'Permanent', r'(?:this\.)?getTargetPointer\(\)\.getFirstTargetPermanentOrLKI\(game,source\)', ('perm_lki', 'target')),
    (r'Permanent', r'game\.getPermanentOrLKIBattlefield\(' + FIRST_TARGET + r'\)', ('perm_lki', 'target')),
    (r'Card', r'game\.getCard\(' + FIRST_TARGET + r'\)', ('card', 'target')),
    (r'Player', r'game\.getPlayer\(' + SECOND_TARGET + r'\)', ('player', 'second')),
    (r'Permanent', r'game\.getPermanent\(' + SECOND_TARGET + r'\)', ('perm', 'second')),
    (r'Card', r'game\.getCard\(' + SECOND_TARGET + r'\)', ('card', 'second')),
    (r'Player', r'game\.getPlayer\(game\.getControllerId\(' + FIRST_TARGET + r'\)\)', ('player', 'target.controller')),
    (r'Permanent', r'source\.getSourcePermanentIfItStillExists\(game\)', ('source', 'source')),
    (r'Permanent', r'game\.getPermanent\(' + SOURCE_ID + r'\)', ('source', 'source')),
    (r'Permanent', r'source\.getSourcePermanentOrLKI\(game\)', ('source_lki', 'source')),
    (r'Permanent', r'game\.getPermanentOrLKIBattlefield\(' + SOURCE_ID + r'\)', ('source_lki', 'source')),
    (r'(?:MageObject|Card)', r'source\.getSourceObject\(game\)', ('source_object', None)),
    (r'(?:MageObject|Card)', r'game\.getObject\(source\)', ('source_object', None)),
    (r'(?:MageObject|Card)', r'source\.getSourceObjectIfItStillExists\(game\)', ('source_object', None)),
]

PLAYERS = ('you', 'player')
OBJECTS = ('perm', 'perm_lki', 'card')
SOURCES = ('source', 'source_lki')

# `card.isLand(game)`: the filter the engine checks
TYPE_CHECKS = {'isLand': [{'p': 'type', 't': 'LAND'}], 'isCreature': [{'p': 'type', 't': 'CREATURE'}],
               'isArtifact': [{'p': 'type', 't': 'ARTIFACT'}], 'isEnchantment': [{'p': 'type', 't': 'ENCHANTMENT'}],
               'isPlaneswalker': [{'p': 'type', 't': 'PLANESWALKER'}], 'isInstant': [{'p': 'type', 't': 'INSTANT'}],
               'isSorcery': [{'p': 'type', 't': 'SORCERY'}],
               'isInstantOrSorcery': [{'p': 'or', 'xs': [{'p': 'type', 't': 'INSTANT'}, {'p': 'type', 't': 'SORCERY'}]}],
               'isPermanent': [{'p': 'or', 'xs': [{'p': 'type', 't': t} for t in
                                                  ('ARTIFACT', 'CREATURE', 'ENCHANTMENT', 'LAND', 'PLANESWALKER', 'BATTLE')]}],
               'isSnow': [{'p': 'supertype', 's': 'SNOW'}], 'isLegendary': [{'p': 'supertype', 's': 'LEGENDARY'}],
               'isBasic': [{'p': 'supertype', 's': 'BASIC'}]}

# `player.moveCards(card, zone, source, game)` -> MoveTargetCardsEffect's zone
ZONES = {'Zone.HAND': 'HAND', 'Zone.EXILED': 'EXILED', 'Zone.GRAVEYARD': 'GRAVEYARD', 'Zone.BATTLEFIELD': 'BATTLEFIELD'}


class Body:
    """One `apply` body read into engine effects"""

    def __init__(self, t, fields=None):
        self.t = t
        self.refs = {}     # local -> (kind, slot)
        self.effects = {}  # effect locals: name -> [effect dict, slot it points at]
        self.ints = set()  # int locals already worked out (the nested sequence's X)
        self.fields = fields or {}
        # the body reads the spell's own X somewhere: no other number can be the sequence's X
        self.real_x = False
        self.in_loop = False
        self.slot_count = [0]
        # filters the body builds (`FilterCard filter = new FilterCard(); filter.add(…);`), by local name
        self.local_filters = {}
        # costs a player may pay (`Cost cost = new GenericManaCost(2)`), by local name
        self.costs = {}

    def pile(self, expr):
        """a pile expression (a `Cards` local, a chosen target's `getTargets()`) -> its slot, or None"""
        expr = expr.strip()
        m = re.fullmatch(r'new CardsImpl\((.*)\)', expr)
        if m:
            return self.pile(m.group(1))
        m = re.fullmatch(r'(\w+)(?:\.getTargets\(\)|\.getCards\(game\))?', expr)
        if m and self.kind(m.group(1)) == 'pile':
            return self.ref(m.group(1))[1]
        return None

    def card_filter(self, expr):
        expr = expr.strip()
        if expr in self.local_filters:
            return self.local_filters[expr]
        return self.t.filter(expr)

    def target_spec(self, rhs):
        """`new TargetCardInLibrary(0, 1, filter)` and the like -> (min, max, filter, zone), or None"""
        rhs = rhs.strip()
        notarget = False
        while (mm := re.fullmatch(r'(.*)\.(withChooseHint\("[^"]*"\)|withNotTarget\(true\))', rhs)) and _balanced(mm.group(1)):
            rhs, notarget = mm.group(1), notarget or mm.group(2).startswith('withNotTarget')
        m = re.fullmatch(r'new TargetCardInGraveyard\((.*)\)', rhs)
        if m:
            spec = self.target_spec('new TargetCardInYourGraveyard(%s)' % m.group(1))
            if spec:
                spec['zone'] = 'graveyards'
            return spec
        m = re.fullmatch(r'new TargetPermanent\((.*),true\)', rhs)
        if m and len(_args(m.group(1))) in (1, 3):
            rhs, notarget = 'new TargetPermanent(%s)' % m.group(1), True
        m = re.fullmatch(r'new TargetSacrifice\((.*)\)', rhs)
        if m:
            spec = self.target_spec('new TargetCardInHand(%s)' % m.group(1))
            if spec:
                spec['zone'] = 'battlefield'
                spec['filter'] = spec['filter'] + [{'p': 'controller', 'who': 'you'}]
            return spec
        m = re.fullmatch(r'new TargetControlled(Creature)?Permanent\((.*)\)', rhs)
        if m:
            args = _args(m.group(2))
            if m.group(1):
                if len(args) not in (0, 2):
                    return None
                args = args + ['StaticFilters.FILTER_CONTROLLED_CREATURE']
            spec = self.target_spec('new TargetCardInHand(%s)' % ','.join(args))
            if spec:
                spec['zone'] = 'battlefield'
                spec['filter'] = spec['filter'] + [{'p': 'controller', 'who': 'you'}]
            return spec
        m = re.fullmatch(r'new TargetPermanent\((.*)\)', rhs)
        if m and notarget:
            spec = self.target_spec('new TargetCardInHand(%s)' % m.group(1))
            if spec:
                spec['zone'] = 'battlefield'
            return spec
        m = re.fullmatch(r'new (TargetCardInLibrary|TargetCard|TargetCardInHand|TargetCardInYourGraveyard|TargetCardInExile)\((.*)\)', rhs)
        if not m:
            return None
        cls, args = m.group(1), _args(m.group(2))
        zone = {'TargetCardInHand': 'hand', 'TargetCardInYourGraveyard': 'graveyard', 'TargetCardInLibrary': 'library',
                'TargetCardInExile': 'exile'}.get(cls)
        if cls == 'TargetCard':
            # (zone[, filter]) or (min, max, zone, filter)
            zi = next((i for i, a in enumerate(args) if a.startswith('Zone.')), None)
            if zi is None:
                return None
            zone = {'Zone.LIBRARY': 'library', 'Zone.HAND': 'hand', 'Zone.GRAVEYARD': 'graveyard',
                    'Zone.EXILED': 'exile', 'Zone.ALL': 'any'}.get(args[zi])
            if zone is None:
                return None
            args = args[:zi] + args[zi + 1:]
        lo, hi, f = 1, 1, []
        if len(args) == 1:
            f = self.card_filter(args[0])
        elif len(args) == 2:
            lo, hi = args[0], args[1]
        elif len(args) == 3:
            lo, hi, f = args[0], args[1], self.card_filter(args[2])
        elif args:
            return None
        hi = 99 if hi == 'Integer.MAX_VALUE' else hi
        if isinstance(lo, str):
            if not re.fullmatch(r'\d+', lo):
                return None
            lo = int(lo)
        if isinstance(hi, str):
            hi = self.amount(hi)
            if isinstance(hi, tuple):
                raise Unsupported('choice of a worked-out number')
        return {'min': lo, 'max': hi, 'filter': f, 'zone': zone}

    def bound(self, bind, slot_kind, name, rest):
        """the rest of the body with `name` naming the slot `bind` makes"""
        pre = bind.pop('pre', None)
        inner = self.child()
        inner.refs[name] = (slot_kind, bind['name'])
        effects = inner.nodes(rest)
        if pre:
            # (asked even when nothing follows: the question is part of the game)
            return [{'class': 'Effects', 'bind': pre, 'effects': [{'class': 'Effects', 'effects': effects, 'bind': bind}]}]
        return [{'class': 'Effects', 'effects': effects, 'bind': bind}]

    def search_call(self, st):
        """`p.searchLibrary(target, source, game)` -> (the target local, its bind), or None"""
        m = re.fullmatch(r'(\w+)\.searchLibrary\((\w+),source,game\)', st.strip())
        if not m or self.kind(m.group(1)) != 'you':
            return None
        r = self.ref(m.group(2))
        if not r or r[0] != 'tspec' or r[1]['zone'] != 'library':
            return None
        spec = r[1]
        return m.group(2), {'name': self.new_slot('found'), 'what': 'choose', 'zone': 'library', 'filter': spec['filter'],
                            'min': spec['min'], 'max': spec['max']}

    def choose_call(self, st):
        """`p.choose(outcome, cards, target, source, game)` (or `chooseTarget`, or without the pile) -> (the target
        local, its bind), or None"""
        m = re.fullmatch(r'(\w+)\.(choose|chooseTarget)\((.*)\)', st.strip())
        if not m or self.kind(m.group(1)) not in PLAYERS or not _balanced(m.group(3)):
            return None
        args = _args(m.group(3))
        if len(args) not in (4, 5) or args[-2:] != ['source', 'game']:
            return None
        if not re.fullmatch(r'Outcome\.\w+|outcome', args[0]):
            return None
        tname = args[-3]
        r = self.ref(tname)
        if not r or r[0] != 'tspec':
            return None
        spec = r[1]
        who = self.ref(m.group(1))[1]
        bind = {'name': self.new_slot('chosen'), 'what': 'choose', 'filter': spec['filter'], 'min': spec['min'],
                'max': spec['max']}
        if who != 'you':
            bind['who'] = who
        if len(args) == 5:
            pile = self.pile(args[1])
            zm = re.fullmatch(r'(\w+)\.(getHand|getGraveyard)\(\)', args[1])
            if pile is None and zm and self.kind(zm.group(1)) in PLAYERS:
                # a zone passed as the cards: an empty one isn't asked about (bound as a pile first)
                pre = self.of({'name': self.new_slot('cards'), 'what': 'hand' if zm.group(2) == 'getHand' else 'graveyard'},
                              zm.group(1))
                bind['from'] = pre['name']
                bind['pre'] = pre
                return tname, bind
            if pile is None:
                return None
            bind['from'] = pile
        else:
            # a hand or graveyard target: the chooser's own zone
            if spec['zone'] == 'graveyards' or spec['zone'] == 'battlefield':
                bind['zone'] = spec['zone']
                if m.group(2) == 'chooseTarget':
                    bind['quiet'] = True
                return tname, bind
            if spec['zone'] not in ('hand', 'graveyard'):
                return None
            bind['zone'] = spec['zone']
            if who != 'you':
                bind['of'] = who
            if m.group(2) == 'chooseTarget':
                bind['quiet'] = True
        return tname, bind

    def new_slot(self, base):
        self.slot_count[0] += 1
        return '%s%d' % (base, self.slot_count[0])

    def child(self):
        b = Body(self.t, self.fields)
        b.refs, b.effects, b.ints = dict(self.refs), dict(self.effects), set(self.ints)
        b.real_x, b.in_loop, b.slot_count = self.real_x, self.in_loop, self.slot_count
        b.local_filters = dict(self.local_filters)
        b.costs = dict(self.costs)
        return b

    # ---- expressions ----

    def ref(self, expr):
        """a local (or an expression) naming a player, permanent or card -> (kind, slot), or None"""
        expr = expr.strip()
        if expr in self.refs:
            return self.refs[expr]
        for _, rhs, kind in REFS:
            if re.fullmatch(rhs, expr):
                return kind
        return None

    def kind(self, expr):
        r = self.ref(expr)
        return r[0] if r else None

    def player_id(self, expr):
        """a player's id -> (kind, slot) of the player"""
        expr = expr.strip()
        if re.fullmatch(CONTROLLER_ID, expr):
            return ('you', 'you')
        if expr in self.refs and self.refs[expr][0] in PLAYERS:
            return self.refs[expr]
        m = re.fullmatch(r'(\w+)\.(getControllerId|getId|getOwnerId)\(\)', expr)
        if m:
            r = self.ref(m.group(1))
            if r and m.group(2) == 'getId' and r[0] in PLAYERS:
                return r
            if r and m.group(2) == 'getControllerId' and r[0] in ('perm', 'perm_lki'):
                return ('player', r[1] + '.controller')
            if r and m.group(2) == 'getControllerId' and r[0] in SOURCES:
                return ('you', 'you')
        if re.fullmatch(FIRST_TARGET, expr):
            return ('player', 'target')
        return None

    def amount(self, expr):
        """an int expression -> an engine Amount, or ('quantity', q) for one the sequence works out first"""
        COUNTERS = T.COUNTERS
        expr = expr.strip()
        while re.fullmatch(r'\((.*)\)', expr) and _balanced(expr[1:-1]):
            expr = expr[1:-1]
        if re.fullmatch(r'\d+', expr):
            return int(expr)
        if expr in self.ints:
            return {'v': 'x'}
        if expr in self.fields:
            return self.amount(self.fields[expr])
        if re.fullmatch(r'CardUtil\.getSourceCostsTag\(game,source,"X",0\)|source\.getManaCostsToPay\(\)\.getX\(\)', expr):
            return {'v': 'x'}
        m = re.fullmatch(r'(.*)\.calculate\(game,source,this\)', expr)
        if m:
            return self.t.amount(m.group(1))
        m = re.fullmatch(r'(\w+)\.(getPower|getToughness)\(\)\.getValue\(\)', expr)
        if m:
            r = self.ref(m.group(1))
            what = 'power' if m.group(2) == 'getPower' else 'toughness'
            if r and r[0] in OBJECTS:
                return ('quantity', {'q': what, 'of': r[1]})
            if r and r[0] in SOURCES:
                return {'v': 'source_' + what}
        m = re.fullmatch(r'(\w+)\.(?:size\(\)|count\((.*),game\))', expr)
        if m and self.pile(m.group(1)):
            q = {'q': 'count', 'of': self.pile(m.group(1))}
            if m.group(2):
                q['filter'] = self.card_filter(m.group(2))
            return ('quantity', q)
        m = re.fullmatch(r'(\w+)\.getManaValue\(\)', expr)
        if m and self.kind(m.group(1)) in OBJECTS:
            return ('quantity', {'q': 'mana_value', 'of': self.ref(m.group(1))[1]})
        m = re.fullmatch(r'(\w+)\.getHand\(\)\.size\(\)', expr)
        if m and self.kind(m.group(1)) == 'you':
            return {'v': 'cards_in_hand'}
        m = re.fullmatch(r'(\w+)\.getLife\(\)', expr)
        if m and self.kind(m.group(1)) == 'you':
            return {'v': 'controller_life'}
        m = re.fullmatch(r'game\.getBattlefield\(\)\.count\((.*),' + CONTROLLER_ID + r',source,game\)', expr)
        if m:
            return {'v': 'permanents', 'filter': self.t.filter(m.group(1))}
        m = re.fullmatch(r'(\w+)\.getCounters\(game\)\.getCount\((CounterType\.\w+)\)', expr)
        if m and self.kind(m.group(1)) in SOURCES and m.group(2) in COUNTERS:
            return {'v': 'source_counters', 'counter': COUNTERS[m.group(2)]}
        raise Unsupported('amount ' + expr)

    def counter(self, expr):
        COUNTERS = T.COUNTERS
        m = re.fullmatch(r'(CounterType\.\w+)\.createInstance\((.*)\)', expr.strip())
        if not m or m.group(1) not in COUNTERS:
            raise Unsupported('counter ' + expr)
        n = self.amount(m.group(2)) if m.group(2) else 1
        return COUNTERS[m.group(1)], n

    def effect_expr(self, expr):
        """`new X(…)` with setters (or an effect local) -> (effect, the slot it points at)"""
        canonical = T.canonical
        expr = canonical(expr)
        pointer = None
        m = re.fullmatch(r'(.*)\.setTargetPointer\((.*)\)', expr)
        if m and _balanced(m.group(1)) and _balanced(m.group(2)):
            expr, pointer = m.group(1), self.pointer(m.group(2))
        m = re.fullmatch(r'(.*)\.(?:setText|concatBy|withTargetDescription)\("[^"]*"\)', expr)
        if m and _balanced(m.group(1)):
            return self.effect_expr(m.group(1))[0], pointer
        if expr in self.effects:
            e, p = self.effects[expr]
            return e, pointer or p
        m = re.fullmatch(r'\((.*)\)', expr)
        if m and _balanced(m.group(1)):
            return self.effect_expr(m.group(1))
        if not expr.startswith('new '):
            raise Unsupported('effect expression ' + expr)
        return self.t.effect(self.substitute(expr)), pointer

    def substitute(self, expr):
        """int locals (worked out as the nested sequence's X) as `GetXValue.instance` where an effect takes a
        number"""
        for v in self.ints:
            expr = re.sub(r'(?<![\w.])' + v + r'(?![\w(])', 'GetXValue.instance', expr)
        return expr

    def pointer(self, expr):
        """a target pointer -> the slot it names (None: the ability's targets)"""
        expr = expr.strip()
        if expr == 'new SecondTargetPointer()':
            return 'second'
        if expr == 'new FirstTargetPointer()':
            return None
        m = re.fullmatch(r'new FixedTarget\((.*?)(?:,game)?\)', expr)
        if not m:
            raise Unsupported('target pointer ' + expr)
        what = m.group(1)
        r = self.ref(what) or self.player_id(what)
        if r is None and re.fullmatch(FIRST_TARGET, what):
            return None
        if r is None or r[1] is None:
            raise Unsupported('target pointer ' + expr)
        return None if r[1] == 'target' else r[1]

    # ---- statements ----

    def in_place(self, node):
        """a statement or `if` that reads a hand, a graveyard or a library's top cards in place (`p.getHand().isEmpty()`,
        `pile.addAll(p.getLibrary().getTopCards(game, 3))`) -> (a pile local for it, the node reading that local),
        or None"""
        text = node[1]
        m = re.search(r'(?<![\w.])(\w+)\.(getHand|getGraveyard)\(\)(?=\.(?:isEmpty|count|size|getCards)\()', text)
        if not m:
            m = re.search(r'(?<=\.addAll\()(\w+)\.getLibrary\(\)\.getTopCards\(game,(\w+)\)(?=\)$)', text)
            m = m or re.search(r'(?<=\.addAllCards\()(\w+)\.getLibrary\(\)\.getTopCards\(game,(\w+)\)(?=\)$)', text)
            m = m or re.search(r'(?<=\.moveCards\()(\w+)\.getLibrary\(\)\.getTopCards\(game,(\w+)\)(?=,)', text)
        top = None
        if not m:
            top = re.search(r'(?<=\.add\()(\w+)\.getLibrary\(\)\.getFromTop\(game\)(?=\)$)', text)
            if top and self.kind(top.group(1)) in PLAYERS:
                local = '__card%d' % self.slot_count[0]
                self.slot_count[0] += 1
                return ('stmt', 'Card %s=%s' % (local, top.group(0))), \
                    (node[0], text[:top.start()] + local + text[top.end():]) + tuple(node[2:])
        if not m or self.kind(m.group(1)) not in PLAYERS:
            return None
        if node[0] == 'stmt' and re.match(r'(?:final )?(?:Cards|Set<Card>) \w+=', text):
            return None
        local = '__pile%d' % self.slot_count[0]
        self.slot_count[0] += 1
        decl = ('stmt', 'Cards %s=new CardsImpl(%s)' % (local, m.group(0)))
        new_text = text[:m.start()] + local + text[m.end():]
        new_text = new_text.replace('.addAllCards(', '.addAll(')
        return decl, (node[0], new_text) + tuple(node[2:])

    def nodes(self, nodes):
        """statements -> engine effects (a guard, a bound card or an int local nests the rest)"""
        out = []
        for k, node in enumerate(nodes):
            rest = nodes[k + 1:]
            if node[0] in ('stmt', 'if') and (found := self.in_place(node)):
                return out + self.nodes([found[0], found[1]] + list(rest))
            if node[0] == 'block':
                return out + self.nodes(node[1] + rest)
            if node[0] == 'effects':
                out.extend(node[1])
                continue
            if node[0] == 'loop':
                out.append(self.loop(node[1], node[2]))
                continue
            if node[0] == 'if':
                cond, neg = node[1], False
                if cond.startswith('!') and self.choose_call(cond[1:]):
                    cond, neg = cond[1:], True
                # `if (p.searchLibrary(…))` holds whenever the search happens
                if self.search_call(cond):
                    if not _has_return(node[2]):
                        return out + self.nodes([('stmt', cond)] + list(node[2]) + list(rest))
                    if _ends_with_return(node[2]):
                        return out + self.nodes([('stmt', cond)] + list(node[2]))
                    raise Unsupported('if ' + cond)
                ch = self.choose_call(cond) if not cond.startswith('!') else None
                if ch:
                    t = ch[0]
                    test = ('!' if not neg else '') + t + '.isEmpty()'
                    return out + self.nodes([('stmt', cond), ('if', test, node[2], node[3])] + list(rest))
                return out + self.branch(node, rest)
            done, effects = self.statement(node[1], rest)
            out.extend(effects)
            if done:
                return out
        return out

    def branch(self, node, rest):
        """`if (…) {…} [else {…}]` and the statements after it"""
        _, cond, then, other = node
        ends = self.ends
        parts = _split(cond, '||')
        if len(parts) > 1 and ends(then) and not other and self.null_check(parts[0]) is not None:
            # if (x == null || test) return;  -> if (x == null) return; if (test) return;
            return self.branch(('if', parts[0], then, other),
                               [('if', '||'.join(parts[1:]), then, other)] + list(rest))
        req = self.null_check(cond)
        if req is not None:
            # if (x == null) return …;  the rest needs x
            if ends(then) and not other:
                return self.guarded(req, rest)
            if self.test(cond) is None:
                raise Unsupported('null check ' + cond)
        parts = _split(cond, '&&')
        if len(parts) > 1 and self.not_null(cond) is None:
            # `controller != null && …`: the controller is always there
            kept = [p for p in parts if not (re.fullmatch(r'(\w+)!=null', p) and self.kind(p[:-6]) in ('you', 'source_object'))]
            if len(kept) < len(parts):
                return self.branch(('if', '&&'.join(kept), then, other), rest)
        req = self.not_null(cond)
        if req is not None:
            # if (x != null) { … } [else return …;] …
            if other and not ends(other):
                raise Unsupported('if ' + cond)
            if _ends_with_return(then) or other:
                if rest and not _only_returns(rest):
                    raise Unsupported('if ' + cond)
                return self.guarded(req, then)
            if _has_return(then) and rest and not _only_returns(rest):
                raise Unsupported('if ' + cond)
            return self.guarded(req, then) + self.nodes(rest)
        test = self.test(cond)
        if test is None:
            raise Unsupported('if ' + cond)
        if ends(then) and not other:
            # if (!test) return;  the rest when it holds
            return self.when(_negate(test), rest, [])
        if not rest or _only_returns(rest):
            return self.when(test, then, other)
        if not _has_return(then) and not _has_return(other):
            return self.when(test, then, other) + self.nodes(rest)
        # a branch that may return: the rest goes on in each branch that doesn't (`if (test) { …; return; }` the
        # rest otherwise; a return inside a nested if stops only that path)
        then_ = list(then) if _ends_with_return(then) else list(then) + list(rest)
        other_ = list(other) if _ends_with_return(other) else list(other) + list(rest)
        return self.when(test, then_, other_)

    def when(self, test, then, other):
        """a test (from `test`) with its two branches -> effects"""
        test = _pay_test(test)
        kind = test[0]
        if kind == 'pay':
            raise Unsupported('pay without asking')
        if kind == 'not':
            return self.when(test[1], other, then)
        if kind == 'and':
            # a && b: then only when both hold; the other branch when either doesn't
            return self.when(test[1], [('effects', self.when(test[2], then, other))], other)
        if kind == 'or':
            return self.when(test[1], then, [('effects', self.when(test[2], then, other))])
        yes, no = self.child().nodes(then), self.child().nodes(other)
        if not yes and not no:
            return []
        if kind == 'condition':
            out = {'class': 'ConditionalOneShotEffect', 'condition': test[1]}
            if yes:
                out['effect'] = _one(yes)
            if no:
                out['otherwise'] = _one(no)
            return [out]
        out = {'class': 'Effects', 'effects': yes}
        if no:
            out['otherwise'] = no
        if kind == 'filter':
            out['when'] = {'slot': test[1], 'filter': test[2]}
        elif kind == 'any':
            out['when'] = {'slot': test[1], 'filter': test[2], 'any': True}
        elif kind == 'count':
            out['when'] = {'slot': test[1], 'filter': test[2], 'cmp': test[3], 'n': test[4]}
        elif kind == 'may':
            out['may'] = {'message': test[1]}
            if test[2] and test[2] != 'you':
                out['may']['who'] = test[2]
            if len(test) > 3:
                out['may']['pay'] = test[3]
        return [out]

    def test(self, cond):
        """an if's condition -> ('condition', c) | ('filter', slot, filter) | ('may', message, who) |
        ('not', t) | ('and', a, b) | ('or', a, b), or None"""
        cond = cond.strip()
        while re.fullmatch(r'\((.*)\)', cond) and _balanced(cond[1:-1]):
            cond = cond[1:-1]
        for op, kind in (('||', 'or'), ('&&', 'and')):
            parts = _split(cond, op)
            if len(parts) > 1:
                tests = [self.test(p) for p in parts]
                if any(t is None for t in tests):
                    return None
                out = tests[-1]
                for t in reversed(tests[:-1]):
                    out = (kind, t, out)
                return out
        if cond.startswith('!'):
            t = self.test(cond[1:])
            return None if t is None else ('not', t)
        m = re.fullmatch(r'(.*)\.apply\(game,source\)', cond)
        if m and _balanced(m.group(1)):
            try:
                return ('condition', self.t.condition(m.group(1)))
            except Exception:  # noqa: BLE001: the translator's own Unsupported
                return None
        m = re.fullmatch(r'(\w+)\.(is\w+)\(game\)', cond)
        if m and m.group(2) in TYPE_CHECKS and self.kind(m.group(1)) in OBJECTS:
            return ('filter', self.ref(m.group(1))[1], TYPE_CHECKS[m.group(2)])
        m = re.fullmatch(r'(\w+)\.hasSubtype\(SubType\.(\w+),game\)', cond)
        if m and self.kind(m.group(1)) in OBJECTS and m.group(2) in T.subtype_names():
            return ('filter', self.ref(m.group(1))[1], [{'p': 'subtype', 's': T.subtype_names()[m.group(2)]}])
        m = re.fullmatch(r'(\w+)\.chooseUse\((.*)\)', cond)
        if m and self.kind(m.group(1)) in PLAYERS and _balanced(m.group(2)):
            args = _args(m.group(2))
            # (outcome, message[, secondMessage, trueText, falseText], source, game)
            if len(args) in (4, 7) and args[-2:] == ['source', 'game'] and re.fullmatch(r'Outcome\.\w+|outcome', args[0]):
                return ('may', _text(args[1]), self.ref(m.group(1))[1])
        m = re.fullmatch(r'(\w+)\.pay\(source,game,source,(\w+)\.getId\(\),false\)', cond)
        if m and m.group(1) in self.costs and self.kind(m.group(2)) in PLAYERS:
            return ('pay', self.costs[m.group(1)], self.ref(m.group(2))[1])
        m = re.fullmatch(r'(\w+(?:\.getTargets\(\))?)\.isEmpty\(\)', cond)
        if m and self.pile(m.group(1)):
            return ('not', ('any', self.pile(m.group(1)), []))
        m = re.fullmatch(r'(\w+)\.(?:size\(\)>0|count\((.*),game\)>0)', cond)
        if m and self.pile(m.group(1)):
            return ('any', self.pile(m.group(1)), self.card_filter(m.group(2)) if m.group(2) else [])
        m = re.fullmatch(r'(\w+)\.getCards\((.*),game\)\.isEmpty\(\)', cond)
        if m and self.pile(m.group(1)):
            return ('not', ('any', self.pile(m.group(1)), self.card_filter(m.group(2))))
        if cond in self.refs and self.refs[cond][0] == 'flag':
            return ('any', self.refs[cond][1], [])
        # a pile's size compared (a `switch` on it too)
        m = re.fullmatch(r'(\w+)\.(?:size\(\)|count\((.*),game\))(==|!=|<=|>=|<|>)(\d+)', cond)
        if m and self.pile(m.group(1)):
            f = self.card_filter(m.group(2)) if m.group(2) else []
            return ('count', self.pile(m.group(1)), f, m.group(3), int(m.group(4)))
        # a card a choice bound (or none)
        m = re.fullmatch(r'(\w+)(==|!=)null', cond)
        if m and self.kind(m.group(1)) == 'card' and self.ref(m.group(1))[1] and not self.ref(m.group(1))[1].startswith('target'):
            t = ('any', self.ref(m.group(1))[1], [])
            return t if m.group(2) == '!=' else ('not', t)
        m = re.fullmatch(r'(\w+)\.isChosen\(game\)', cond)
        if m and self.pile(m.group(1)):
            return ('any', self.pile(m.group(1)), [])
        m = re.fullmatch(r'(\w+)\.match\((\w+),game\)', cond)
        if m and m.group(1) in self.local_filters and self.kind(m.group(2)) in OBJECTS:
            return ('filter', self.ref(m.group(2))[1], self.local_filters[m.group(1)])
        return None

    def ends(self, nodes):
        """a branch that only stops: `return …;` (or `continue;` in a loop)"""
        return _returns(nodes) or (self.in_loop and len(nodes) == 1 and nodes[0] == ('stmt', 'continue'))

    def guarded(self, req, rest):
        inner = self.nodes(rest)
        req = [r for r in req if r]
        if not req:
            return inner
        if not inner:
            return []
        return [{'class': 'Effects', 'effects': inner, 'require': req}]

    def requirement(self, name, is_=''):
        r = self.ref(name)
        if r is None:
            return 'unknown'
        kind, slot = r
        if kind in ('you', 'source_object'):
            return None
        if kind == 'source':
            return {'slot': 'source', 'is': 'creature'} if is_ == 'creature' else {'slot': 'source'}
        if kind == 'source_lki':
            return {'slot': 'source_lki'}
        if kind == 'perm':
            return {'slot': slot, 'is': is_ or 'permanent'}
        return {'slot': slot}

    def null_check(self, cond):
        """`a == null || b == null` -> what the engine checks for them, else None"""
        reqs = []
        for p in _split(cond, '||'):
            m = re.fullmatch(r'(\w+)==null', p)
            m2 = re.fullmatch(r'!(\w+)\.isCreature\(game\)', p)
            m3 = re.fullmatch(r'!(\w+)\.getLibrary\(\)\.hasCards\(\)', p)
            if m:
                r = self.requirement(m.group(1))
            elif m2 and self.kind(m2.group(1)) in ('perm', 'source'):
                r = self.requirement(m2.group(1), 'creature')
            elif m3 and self.kind(m3.group(1)) in PLAYERS:
                r = {'slot': self.ref(m3.group(1))[1], 'is': 'library'}
            else:
                return None
            if r == 'unknown':
                return None
            reqs.append(r)
        return reqs

    def not_null(self, cond):
        reqs = []
        for p in _split(cond, '&&'):
            m = re.fullmatch(r'(\w+)!=null', p)
            if not m:
                return None
            r = self.requirement(m.group(1))
            if r == 'unknown':
                return None
            reqs.append(r)
        return reqs

    def loop(self, header, body):
        """`for (UUID id : game.getOpponents(…))`, `getPlayersInRange`, `for (Permanent p : …getActivePermanents(
        filter, …))`: the body once for each, named in a slot of its own"""
        h = re.sub(r'\s*:\s*', ':', header)
        who = r'([\w.]+\(\)|\w+)'
        each = var = kind = None

        def player(expr):
            p = self.player_id(expr)
            return p[1] if p else None
        if (m := re.fullmatch(r'for\((?:final )?UUID (\w+):game\.getOpponents\(' + who + r'\)\)', h)) \
                and player(m.group(2)) == 'you':
            each, var, kind = {'kind': 'opponents'}, m.group(1), 'player'
        elif (m := re.fullmatch(r'for\((?:final )?UUID (\w+):game\.getState\(\)\.getPlayersInRange\(' + who + r',game\)\)', h)) \
                and player(m.group(2)) == 'you':
            each, var, kind = {'kind': 'players'}, m.group(1), 'player'
        elif (m := re.fullmatch(r'for\((?:final )?Permanent (\w+):game\.getBattlefield\(\)\.'
                                r'getActivePermanents\(([\w.]+),' + who + r'(?:,source)?,game\)\)', h)) \
                and player(m.group(3)) == 'you':
            each, var, kind = {'kind': 'permanents', 'filter': self.t.filter(m.group(2))}, m.group(1), 'perm'
        elif (m := re.fullmatch(r'for\((?:final )?Permanent (\w+):game\.getBattlefield\(\)\.'
                                r'getAllActivePermanents\(([\w.]+),' + who + r',game\)\)', h)) and player(m.group(3)):
            # the permanents of the filter that player controls
            each, var, kind = {'kind': 'permanents', 'filter': self.t.filter(m.group(2))}, m.group(1), 'perm'
            if player(m.group(3)) != 'you':
                each['controller'] = player(m.group(3))
            else:
                each['filter'] = each['filter'] + [{'p': 'controller', 'who': 'you'}]
        elif (m := re.fullmatch(r'for\((?:final )?(?:Card|UUID) (\w+):(\w+)(?:\.getCards\(game\)|\.getTargets\(\))?\)', h)) \
                and self.kind(m.group(2)) == 'pile':
            each, var, kind = {'kind': 'cards', 'from': self.ref(m.group(2))[1]}, m.group(1), 'card'
        if each is None:
            raise Unsupported('loop ' + header)
        # a return in the body ends the whole apply, not one pass
        if _has_return(body):
            raise Unsupported('return in a loop')
        inner = self.child()
        inner.in_loop = True
        each['name'] = self.new_slot('each')
        inner.refs[var] = (kind, each['name'])
        effects = inner.nodes(body)
        if not effects:
            raise Unsupported('empty loop')
        return {'class': 'Effects', 'effects': effects, 'each': each}

    def statement(self, st, rest):
        """one statement -> (stop here, effects); the rest goes into a nested sequence when it needs one"""
        st = re.sub(r'^final ', '', st)
        if st in ('return true', 'return false', 'return') or (self.in_loop and st == 'continue'):
            return True, []
        m = re.fullmatch(r'([\w<>]+) (\w+)=(.*)', st)
        if m:
            return self.local(*m.groups(), rest)
        # effect locals' setters
        m = re.fullmatch(r'(\w+)\.setTargetPointer\((.*)\)', st)
        if m and m.group(1) in self.effects:
            self.effects[m.group(1)][1] = self.pointer(m.group(2))
            return False, []
        m = re.fullmatch(r'(\w+)\.setText\(.*\)', st)
        if m and m.group(1) in self.effects:
            return False, []
        # nothing the engine keeps (texts, logs, what a player looks at)
        if re.fullmatch(r'game\.(?:processAction\(\)|getState\(\)\.processAction\(game\)|informPlayers\(.*\))', st):
            return False, []
        # a choice: what follows sees what was chosen
        ch = self.choose_call(st) or self.search_call(st)
        if ch:
            tname, bind = ch
            return True, self.bound(bind, 'pile', tname, rest)
        m = re.fullmatch(r'(\w+)\.(revealCards|lookAtCards)\((.*)\)', st)
        if m and self.kind(m.group(1)) in PLAYERS:
            args = _args(m.group(3))
            pile = self.pile(args[1]) if len(args) >= 3 else None
            if pile and self.kind(m.group(1)) == 'you':
                e = {'class': 'RevealTargetCardsEffect' if m.group(2) == 'revealCards' else 'LookAtTargetCardsEffect'}
                return False, [{'class': 'Effects', 'effects': [e], 'to': pile}]
            return False, []
        # a pile's own calls
        m = re.fullmatch(r'(\w+)\.(add|remove|addAll|removeAll)\((.*)\)', st)
        if m and self.kind(m.group(1)) == 'pile' and _balanced(m.group(3)):
            what = m.group(3)
            slot = self.pile(what)
            if slot is None:
                r = self.ref(what)
                if r and r[0] in OBJECTS:
                    slot = r[1]
            if slot is None:
                raise Unsupported('statement ' + st)
            cls = 'PileAddEffect' if m.group(2).startswith('add') else 'PileRemoveEffect'
            return False, [{'class': cls, 'pile': self.ref(m.group(1))[1], 'slot': slot}]
        m = re.fullmatch(r'(\w+)\.retainZone\(Zone\.(LIBRARY|HAND|GRAVEYARD|EXILED|BATTLEFIELD),game\)', st)
        if m and self.kind(m.group(1)) == 'pile':
            return False, [{'class': 'PileRetainZoneEffect', 'pile': self.ref(m.group(1))[1], 'zone': m.group(2)}]
        m = re.fullmatch(r'(\w+)\.shuffleLibrary\(source,game\)', st)
        if m and self.kind(m.group(1)) in PLAYERS:
            if self.kind(m.group(1)) == 'you':
                return False, [{'class': 'ShuffleLibrarySourceEffect'}]
            return False, [self.pointed({'class': 'ShuffleLibraryTargetEffect'}, self.ref(m.group(1))[1])]
        m = re.fullmatch(r'(\w+)\.(?:withChooseHint\("[^"]*"\)|setNotTarget\(true\)|withNotTarget\(true\))', st)
        if m and self.kind(m.group(1)) == 'tspec':
            return False, []
        m = re.fullmatch(r'(\w+)\.add\((\w+)\)', st)
        if m and self.kind(m.group(1)) == 'cards_empty' and self.kind(m.group(2)) in OBJECTS:
            self.refs[m.group(1)] = self.ref(m.group(2))
            return False, []
        m = re.fullmatch(r'(\w+)\.add\(.*\)', st)
        if m and m.group(1) in self.local_filters:
            return False, []
        ret = False
        m = re.fullmatch(r'return (.*)', st)
        if m:
            st, ret = m.group(1), True
            parts = _split(st, '&&')
            if len(parts) > 1:
                # `return x != null && …`: each null check guards the calls after it
                out = []
                for p in reversed(parts):
                    mm = re.fullmatch(r'(\w+)!=null', p)
                    if mm and self.ref(mm.group(1)):
                        req = self.requirement(mm.group(1))
                        if req and out:
                            out = [{'class': 'Effects', 'effects': out, 'require': [req]}]
                        continue
                    out = self.action(p) + out
                return True, out
        return ret, self.action(st)

    def local(self, typ, name, rhs, rest):
        for ltyp, lrhs, kind in REFS:
            if re.fullmatch(ltyp, typ) and re.fullmatch(lrhs, rhs):
                self.refs[name] = kind
                return False, []
        if typ == 'UUID' and (p := self.player_id(rhs)):
            self.refs[name] = p
            return False, []
        if typ == 'Player' and (mm := re.fullmatch(r'game\.getPlayer\((.*)\)', rhs)) and (p := self.player_id(mm.group(1))):
            self.refs[name] = p
            return False, []
        # the top card of a library: a slot of its own, for what follows
        mm = re.fullmatch(r'(\w+)\.getLibrary\(\)\.getFromTop\(game\)', rhs)
        if typ == 'Card' and mm and self.kind(mm.group(1)) in PLAYERS:
            slot = self.new_slot('card')
            bind = {'name': slot, 'what': 'top_card'}
            of = self.ref(mm.group(1))[1]
            if of != 'you':
                bind['of'] = of
            inner = self.child()
            inner.refs[name] = ('card', slot)
            effects = inner.nodes(rest)
            return True, [{'class': 'Effects', 'effects': effects, 'bind': bind}] if effects else []
        # a pile: the rest of the body sees it
        if typ in ('Cards', 'Set<Card>'):
            bind = self.pile_bind(rhs, rest, name)
            if bind:
                bind['name'] = self.new_slot('cards')
                return True, self.bound(bind, 'pile', name, rest)
        # a choice of cards to come
        if re.fullmatch(r'Target(?:Card\w*|Permanent|Sacrifice|ControlledCreaturePermanent|ControlledPermanent)?', typ) \
                and (spec := self.target_spec(rhs)):
            self.refs[name] = ('tspec', spec)
            return False, []
        # the first card chosen
        rhs = re.sub(r'^\w+\.(?:getHand\(\)|getGraveyard\(\)|getLibrary\(\)\.getCard)\.?(?:get)?\((\w+)\.getFirstTarget\(\),game\)$',
                     r'game.getCard(\1.getFirstTarget())', rhs)
        mm = re.fullmatch(r'(?:game\.getCard\((\w+)\.getFirstTarget\(\)\)|(\w+)\.get\((\w+)\.getFirstTarget\(\),game\))', rhs)
        if typ in ('Card', 'MageObject') and mm:
            t = mm.group(1) or mm.group(3)
            if self.kind(t) == 'pile' and (mm.group(2) is None or self.kind(mm.group(2)) == 'pile'):
                bind = {'name': self.new_slot('card'), 'what': 'first', 'from': self.ref(t)[1]}
                return True, self.bound(bind, 'card', name, rest)
        mm = re.fullmatch(r'game\.getPermanent\((\w+)\.getFirstTarget\(\)\)', rhs)
        if typ == 'Permanent' and mm and self.kind(mm.group(1)) == 'pile':
            bind = {'name': self.new_slot('perm'), 'what': 'first', 'from': self.ref(mm.group(1))[1]}
            return True, self.bound(bind, 'perm', name, rest)
        # a loop's card (or permanent) id
        mm = re.fullmatch(r'game\.(getCard|getPermanent)\((\w+)\)', rhs)
        if typ in ('Card', 'Permanent') and mm and self.kind(mm.group(2)) == 'card' \
                and (typ == 'Card') == (mm.group(1) == 'getCard'):
            self.refs[name] = ('card' if typ == 'Card' else 'perm', self.ref(mm.group(2))[1])
            return False, []
        # a pile of one card (to reveal it, or move it): the card itself
        if typ == 'Cards' and (mm := re.fullmatch(r'new CardsImpl\((\w*)\)', rhs)):
            if not mm.group(1):
                self.refs[name] = ('cards_empty', None)
                return False, []
            r = self.ref(mm.group(1))
            if r and r[0] in OBJECTS:
                self.refs[name] = r
                return False, []
        # a filter built here, with its `add`s later in the body
        mm = re.fullmatch(r'new (Filter\w+)\((.*)\)', rhs)
        if typ.startswith('Filter') and mm:
            filter_object, split_args, subtype_names = T.filter_object, T.split_args, T.subtype_names
            adds = [n[1][len(name) + 5:-1] for n in rest if n[0] == 'stmt' and n[1].startswith(name + '.add(')]
            self.local_filters[name] = filter_object(mm.group(1), split_args(mm.group(2)), adds, subtype_names())
            return False, []
        if typ == 'boolean':
            t = self.test(rhs)
            if t and t[0] == 'filter':
                # (worked out now: the object may have changed by the time the flag is read)
                bind = {'name': self.new_slot('flag'), 'what': 'filtered', 'from': t[1], 'filter': t[2]}
                return True, self.bound(bind, 'flag', name, rest)
        if typ in ('int', 'Integer'):
            a = self.amount(rhs)
            q = a[1] if isinstance(a, tuple) else {'q': 'amount', 'amount': a}
            if self.ints or (self.real_x and a != {'v': 'x'}):
                raise Unsupported('two numbers')
            inner = self.child()
            inner.ints.add(name)
            effects = inner.nodes(rest)
            return True, [{'class': 'Effects', 'effects': effects, 'x': q}] if effects else []
        if re.fullmatch(r'(?:Cost|ManaCosts(?:<ManaCost>)?)', typ):
            mm = re.fullmatch(r'ManaUtil\.createManaCost\((\d+),false\)', rhs)
            pl = re.fullmatch(r'new PayLifeCost\((\d+)\)', rhs)
            if mm:
                self.costs[name] = [{'class': 'ManaCost', 'cost': '{%s}' % mm.group(1)}]
            elif pl:
                self.costs[name] = [{'class': 'PayLifeCost', 'amount': int(pl.group(1))}]
            else:
                c = self.t.cost(rhs)
                if c.get('class') != 'ManaCost' or 'X' in c.get('cost', ''):
                    raise Unsupported('local %s %s=%s' % (typ, name, rhs))
                self.costs[name] = [c]
            return False, []
        if typ == 'Token' and (mm := re.fullmatch(r'new (\w+Token)\(\)', rhs)):
            self.refs[name] = ('token', mm.group(1))
            return False, []
        if re.fullmatch(r'(?:Effect|OneShotEffect|ContinuousEffect|\w+Effect)', typ) and rhs.startswith('new '):
            self.effects[name] = list(self.effect_expr(rhs))
            return False, []
        raise Unsupported('local %s %s=%s' % (typ, name, rhs))

    def pile_bind(self, rhs, rest, name):
        """a pile local's value -> its bind (without the name), or None"""
        inner = re.fullmatch(r'new CardsImpl\((.*)\)', rhs)
        m = re.fullmatch(r'(\w+)\.getLibrary\(\)\.getTopCards\(game,(.*)\)', inner.group(1) if inner else rhs)
        if m and self.kind(m.group(1)) in PLAYERS:
            n = self.amount(m.group(2))
            if isinstance(n, tuple):
                return None
            return self.of({'what': 'top_cards', 'n': n}, m.group(1))
        m = re.fullmatch(r'(\w+)\.millCards\((.*),source,game\)', rhs)
        if m and self.kind(m.group(1)) in PLAYERS:
            n = self.amount(m.group(2))
            if isinstance(n, tuple):
                return None
            return self.of({'what': 'mill', 'n': n}, m.group(1))
        m = re.fullmatch(r'new CardsImpl\((\w+)\.(getHand|getGraveyard)\(\)\)', rhs)
        if m and self.kind(m.group(1)) in PLAYERS:
            return self.of({'what': 'hand' if m.group(2) == 'getHand' else 'graveyard'}, m.group(1))
        if re.fullmatch(r'new CardsImpl\((?:(?:this\.)?getTargetPointer\(\)\.getTargets\(game,source\)|'
                        r'source\.getTargets\(\)\.get\(0\)\.getTargets\(\)|source\.getFirstTarget\(\)\.getTargets\(\))\)', rhs):
            return {'what': 'targets'}
        if re.fullmatch(r'new CardsImpl\(source\.getTargets\(\)\.stream\(\)\.map\(Target::getTargets\)\.flatMap\(Collection::stream\)'
                        r'\.collect\(Collectors\.to(?:Set|List)\(\)\)\)', rhs):
            return {'what': 'all_targets'}
        m = re.fullmatch(r'new CardsImpl\((.*)\)', rhs)
        if m and self.pile(m.group(1)):
            return {'what': 'copy', 'from': self.pile(m.group(1))}
        m = re.fullmatch(r'(\w+)\.getCards\((.*),game\)', rhs)
        if m and self.kind(m.group(1)) == 'pile':
            return {'what': 'filtered', 'from': self.ref(m.group(1))[1], 'filter': self.card_filter(m.group(2))}
        # an empty pile that gets more than one card (else the one-card shortcut below names the card)
        if rhs == 'new CardsImpl()' and (any(re.match(name + r'\.addAll(?:Cards)?\(', n[1]) for n in _flat(rest))
                                         or sum(1 for n in _flat(rest) if n[1].startswith(name + '.add(')) > 1):
            return {'what': 'empty'}
        return None

    def of(self, bind, who):
        p = self.ref(who)[1]
        if p != 'you':
            bind['of'] = p
        return bind

    def action(self, st):
        """a call -> engine effects"""
        st = st.strip()
        # `x.apply(game, source)` (also `… > 0` and friends on a call that returns a number)
        st = re.sub(r'(?:>0|!=0|>=0)$', '', st)
        m = re.fullmatch(r'(.*)\.apply\(game,source\)', st)
        if m and _balanced(m.group(1)):
            return [self.pointed(*self.effect_expr(m.group(1)))]
        m = re.fullmatch(r'game\.addEffect\((.*),source\)', st)
        if m:
            return [self.pointed(*self.effect_expr(m.group(1)))]
        m = re.fullmatch(r'(new \w+Token\(\)|\w+)\.putOntoBattlefield\(([^,]+),game,source(?:,' + CONTROLLER_ID + r')?\)', st)
        if m:
            tok = m.group(1)[4:-2] if m.group(1).startswith('new ') else (
                self.refs[m.group(1)][1] if self.kind(m.group(1)) == 'token' else None)
            if tok:
                return [self.with_quantity({'class': 'CreateTokenEffect', 'token': T.token(tok)}, 'amount',
                                           self.amount(m.group(2)))]
        m = re.fullmatch(r'(\w+)\.(\w+)\((.*)\)', st)
        if not m or not _balanced(m.group(3)):
            raise Unsupported('statement ' + st)
        who, call, args = m.group(1), m.group(2), _args(m.group(3))
        r = self.ref(who)
        if r is None:
            raise Unsupported('statement ' + st)
        kind, slot = r
        player = kind in PLAYERS
        q = self.with_quantity

        def on_player(you_class, target_class, n):
            if kind == 'you':
                return [q({'class': you_class}, 'amount', self.amount(n))]
            return [self.pointed(q({'class': target_class}, 'amount', self.amount(n)), slot)]

        if call == 'drawCards' and len(args) in (2, 3) and player:
            return on_player('DrawCardSourceControllerEffect', 'DrawCardTargetEffect', args[0])
        if call == 'gainLife' and len(args) == 3 and kind == 'you':
            return [q({'class': 'GainLifeEffect'}, 'amount', self.amount(args[0]))]
        if call == 'loseLife' and len(args) == 4 and player:
            return on_player('LoseLifeSourceControllerEffect', 'LoseLifeTargetEffect', args[0])
        if call == 'millCards' and len(args) == 3 and player:
            return on_player('MillCardsControllerEffect', 'MillCardsTargetEffect', args[0])
        if call == 'damage' and len(args) == 4 and re.fullmatch(SOURCE_ID, args[1]) and (player or kind == 'perm'):
            if kind == 'you':
                return [q({'class': 'DamageControllerEffect'}, 'amount', self.amount(args[0]))]
            return [self.pointed(q({'class': 'DamageTargetEffect'}, 'amount', self.amount(args[0])), slot)]
        if call == 'discard' and len(args) == 5 and args[1:3] == ['false', 'false'] and kind == 'player':
            return [self.pointed(q({'class': 'DiscardTargetEffect'}, 'amount', self.amount(args[0])), slot)]
        if call == 'destroy' and kind == 'perm' and (len(args) == 2 or (len(args) == 3 and args[2] in ('true', 'false'))):
            e = {'class': 'DestroyNoRegenerateTargetEffect' if args[-1] == 'true' else 'DestroyTargetEffect'}
            return [self.pointed(e, slot)]
        if call == 'sacrifice' and len(args) == 2 and kind in SOURCES:
            return [{'class': 'SacrificeSourceEffect'}]
        if call == 'sacrifice' and len(args) == 2 and kind == 'perm':
            return [self.pointed({'class': 'SacrificeTargetEffect'}, slot)]
        if call == 'addCounters' and len(args) in (3, 4) and (kind in OBJECTS or kind in SOURCES):
            name, n = self.counter(args[0])
            if kind in SOURCES:
                e = {'class': 'AddCountersSourceEffect', 'counter': {'name': name, 'amount': 1}}
                return [q(e, 'amount', n) if n != 1 else e]
            e = {'class': 'AddCountersTargetEffect', 'counter': {'name': name, 'amount': 1}}
            return [self.pointed(q(e, 'value', n) if n != 1 else e, slot)]
        if call == 'tap' and len(args) == 2 and kind in ('perm', 'source'):
            return [{'class': 'TapSourceEffect'}] if kind == 'source' else [self.pointed({'class': 'TapTargetEffect'}, slot)]
        if call == 'untap' and len(args) == 1 and kind in ('perm', 'source'):
            return [{'class': 'UntapSourceEffect'}] if kind == 'source' else [self.pointed({'class': 'UntapTargetEffect'}, slot)]
        if call in ('moveCards', 'putCardsOnBottomOfLibrary', 'putCardsOnTopOfLibrary') and player and args:
            pile = self.pile(args[0])
            if pile is None and (mm := re.fullmatch(r'(\w+)\.getCards\((.*),game\)', args[0])) and self.kind(mm.group(1)) == 'pile':
                bind = {'name': self.new_slot('cards'), 'what': 'filtered', 'from': self.ref(mm.group(1))[1],
                        'filter': self.card_filter(mm.group(2))}
                inner = self.child()
                inner.refs['__filtered'] = ('pile', bind['name'])
                return [{'class': 'Effects', 'bind': bind,
                         'effects': inner.action(st.replace(args[0], '__filtered', 1))}]
            if pile is not None:
                put = None
                if call == 'moveCards' and len(args) == 4 and args[1] in ZONES:
                    put = {'Zone.HAND': 'HAND', 'Zone.EXILED': 'EXILED', 'Zone.GRAVEYARD': 'GRAVEYARD',
                           'Zone.BATTLEFIELD': 'BATTLEFIELD'}[args[1]]
                elif call == 'moveCards' and len(args) == 8 and args[1] == 'Zone.BATTLEFIELD' and args[5] == 'false':
                    put = 'BATTLEFIELD_TAPPED' if args[4] == 'true' else 'BATTLEFIELD'
                elif call != 'moveCards' and len(args) == 4 and args[1:3] == ['game', 'source'] and args[3] in ('true', 'false'):
                    put = ('TOP_' if call == 'putCardsOnTopOfLibrary' else 'BOTTOM_') + ('ANY' if args[3] == 'true' else 'RANDOM')
                if put and (kind == 'you' or put in ('HAND', 'GRAVEYARD', 'EXILED', 'BOTTOM_RANDOM', 'TOP_RANDOM')):
                    return [{'class': 'Effects', 'effects': [{'class': 'PutTargetCardsEffect', 'put': put}], 'to': pile}]
                raise Unsupported('statement ' + st)
        if call == 'moveCards' and player and args:
            what = self.ref(args[0])
            # (cards, zone, source, game[, tapped, faceDown, byOwner, appliedEffects]); onto the battlefield only
            # under the controller's control
            tapped = len(args) == 8 and args[4] == 'true'
            if what and what[0] in OBJECTS and args[1] in ZONES and (len(args) == 4 or (len(args) == 8 and args[5] == 'false')) \
                    and (kind == 'you' or args[1] != 'Zone.BATTLEFIELD'):
                zone = ZONES[args[1]] + ('_TAPPED' if tapped and args[1] == 'Zone.BATTLEFIELD' else '')
                return [self.pointed({'class': 'MoveTargetCardsEffect', 'zone': zone}, what[1])]
            if what and what[0] == 'source' and len(args) == 4 and args[1] == 'Zone.HAND':
                return [{'class': 'ReturnToHandSourceEffect', 'battlefield_only': True}]
        if call == 'putCardsOnBottomOfLibrary' and len(args) == 4 and player:
            what = self.ref(args[0])
            if what and what[0] in OBJECTS:
                return [self.pointed({'class': 'MoveTargetCardsEffect', 'zone': 'LIBRARY_BOTTOM'}, what[1])]
        raise Unsupported('statement ' + st)

    def with_quantity(self, e, field, a):
        """an effect with its number; one the sequence works out first (the target's power) nests it"""
        if isinstance(a, tuple):
            e[field] = {'v': 'x'}
            if self.real_x or self.ints:
                raise Unsupported('two numbers')
            return {'class': 'Effects', 'effects': [e], 'x': a[1]}
        e[field] = a
        return e

    @staticmethod
    def pointed(e, slot):
        """an effect pointed at a slot (none: the ability's own targets)"""
        if slot is None or slot == 'target':
            return e
        return {'class': 'Effects', 'effects': [e], 'to': slot}


def _pay_test(test):
    """`!p.chooseUse(…) || !cost.pay(…, p, …)` (and `p.chooseUse(…) && cost.pay(…)`): one may that's paid"""
    if test[0] == 'or' and test[1][0] == 'not' and test[2][0] == 'not':
        inner = _pay_test(('and', test[1][1], test[2][1]))
        if inner[0] == 'may':
            return ('not', inner)
    if test[0] == 'and' and test[1][0] == 'may' and len(test[1]) == 3 and test[2][0] == 'pay' and test[1][2] == test[2][2]:
        return ('may', test[1][1], test[1][2], test[2][1])
    return test


def _text(expr):
    """a message built from literals and calls: its literal parts, "it" for a name ("Put " + card.getName() + …)"""
    parts = []
    for p in _split(expr, '+'):
        p = p.strip()
        m = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', p)
        parts.append(m.group(1) if m else 'it' if re.search(r'\.get(?:Log|Id)?Name\(', p) else '')
    return re.sub(r' +', ' ', ''.join(parts)).strip() or '?'


def _negate(t):
    return t[1] if t[0] == 'not' else ('not', t)


def _one(effects):
    return effects[0] if len(effects) == 1 else {'class': 'Effects', 'effects': effects}


def _balanced(text):
    depth, i = 0, 0
    while i < len(text):
        c = text[i]
        if c in '"\'':
            i = _skip_string(text, i)
            continue
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
            if depth < 0:
                return False
        i += 1
    return depth == 0


def _split(text, op):
    out, depth, start, i = [], 0, 0, 0
    while i < len(text):
        c = text[i]
        if c in '"\'':
            i = _skip_string(text, i)
            continue
        if c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif depth == 0 and text.startswith(op, i):
            out.append(text[start:i])
            start = i + len(op)
            i += len(op)
            continue
        i += 1
    out.append(text[start:])
    return [p.strip() for p in out]


def _args(text):
    return [a for a in _split(text, ',') if a] if text.strip() else []


def _returns(nodes):
    return len(nodes) == 1 and nodes[0][0] == 'stmt' and re.fullmatch(r'return(?: true| false)?', nodes[0][1])


def _ends_with_return(nodes):
    return bool(nodes) and nodes[-1][0] == 'stmt' and bool(re.fullmatch(r'return(?: .*)?', nodes[-1][1]))


def _has_return(nodes):
    """a return anywhere in these statements, nested ifs and blocks too (not in a loop's body)"""
    for n in nodes:
        if n[0] == 'stmt' and re.fullmatch(r'return(?: .*)?', n[1]):
            return True
        if n[0] == 'if' and (_has_return(n[2]) or _has_return(n[3] or [])):
            return True
        if n[0] == 'block' and _has_return(n[1]):
            return True
    return False


def _flat(nodes):
    """every statement node, nested ones too"""
    for n in nodes:
        if n[0] == 'stmt':
            yield n
        elif n[0] == 'block':
            yield from _flat(n[1])
        elif n[0] == 'if':
            yield from _flat(n[2])
            yield from _flat(n[3] or [])
        elif n[0] == 'loop':
            yield from _flat(n[2])


def _only_returns(nodes):
    return all(n[0] == 'stmt' and re.fullmatch(r'return(?: true| false)?', n[1]) for n in nodes)


def translate(t, cls, args, source):
    """`new cls(args)` for a class the card declares -> an engine effect, or Unsupported"""
    try:
        return _translate(t, cls, args, source)
    except (Unsupported, T.Unsupported):
        raise
    except Exception as e:  # noqa: BLE001: a body the parser doesn't expect
        raise Unsupported('%s %s' % (type(e).__name__, e))


def _strip_comments(source):
    """the source without comments (an apostrophe in one isn't a char literal), strings kept"""
    return re.sub(r'("(?:[^"\\\n]|\\.)*"|\'(?:[^\'\\\n]|\\.)\')|//[^\n]*|/\*.*?\*/',
                  lambda m: m.group(1) or ' ', source, flags=re.S)


import custom_statics  # noqa: E402: it imports this module's helpers


def _translate(t, cls, args, source):
    source = _strip_comments(source)
    found = class_body(source, cls)
    if not found:
        raise Unsupported('not declared')
    extends, body = found
    if extends in custom_statics.KINDS and not args:
        return custom_statics.translate(t, cls, extends, body, T)
    if extends != 'OneShotEffect':
        raise Unsupported('extends ' + extends)
    if args:
        raise Unsupported('constructor arguments')
    # instance state (fields set in the constructor) isn't read
    fields = {}
    for m in re.finditer(r'\n\s*(?:private |protected |public )?(?:static )?(?:final )?([\w<>]+) (\w+)\s*(=\s*([^;]*))?;', _top_level(body)):
        typ, name, init = m.group(1), m.group(2), m.group(4)
        if typ in ('return',) or '(' in typ:
            continue
        if 'static' in m.group(0) and init is not None:
            if typ.endswith(('Value', 'Count')) or typ == 'DynamicValue':
                fields[name] = init
            continue
        raise Unsupported('field ' + name)
    params, apply = method_body(body, 'apply')
    if apply is None:
        raise Unsupported('no apply')
    if not re.fullmatch(r'\s*Game game\s*,\s*Ability source\s*', params or ''):
        raise Unsupported('apply(' + params + ')')
    b = Body(t, fields)
    b.real_x = bool(re.search(r'"X"|getX\(\)|GetXValue|ManacostVariableValue', apply))
    effects = b.nodes(parse(apply))
    if not effects:
        raise Unsupported('nothing done')
    return effects[0] if len(effects) == 1 else {'class': 'Effects', 'effects': effects}


def _top_level(body):
    """a class body without its methods' and nested classes' bodies (its fields' declarations)"""
    out, depth, i = [], 0, 0
    while i < len(body):
        c = body[i]
        if c in '"\'':
            j = _skip_string(body, i)
            if depth == 0:
                out.append(body[i:j])
            i = j
            continue
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
        elif depth == 0:
            out.append(c)
        i += 1
    return ''.join(out)


def log(card, cls, reason):
    LOG[cls] = (card, str(reason))


def write_log():
    path = os.environ.get('PLAYMAT_APPLY_LOG')
    if path:
        with open(path, 'w') as f:
            for cls, (card, reason) in sorted(LOG.items()):
                f.write('%s\t%s\t%s\n' % (card, cls, reason))

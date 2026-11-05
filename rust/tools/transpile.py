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
DEFAULT_FACTS = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'cards', 'facts.json')

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

# effects: class -> its constructor arguments, each (field name, kind); a kind of None means the argument is ignored
# (a text override: rule texts come from the card facts)
EFFECTS = {
    'DamageTargetEffect': [('amount', 'int'), (None, 'string')],
    'DrawCardSourceControllerEffect': [('amount', 'int')],
    'GainLifeEffect': [('amount', 'int')],
    'DestroyTargetEffect': [],
    'CounterTargetEffect': [],
    'BoostTargetEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration')],
    'BoostSourceEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), (None, 'string')],
    'BoostControlledEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), ('filter', 'filter'),
                              ('exclude_source', 'bool')],
    'AddCountersSourceEffect': [('counter', 'counter')],
    'AddCountersTargetEffect': [('counter', 'counter')],
    'CreateTokenEffect': [('token', 'token'), ('amount', 'int')],
    'DamageWithPowerFromOneToAnotherTargetEffect': [],
    'GainAbilityAllEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter')],
    'ConditionalOneShotEffect': [('effect', 'effect'), ('otherwise', 'effect'), ('condition', 'condition'),
                                 (None, 'string')],
    'DamagePlayersEffect': [('amount', 'int'), ('controller', 'target_controller')],
    'SacrificeSourceEffect': [],
    'DoIfCostPaid': [('effect', 'effect'), ('cost', 'cost')],
    'GainAbilityControlledEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter')],
    'GainAbilityTargetEffect': [('ability', 'ability'), ('duration', 'duration')],
    'GainAbilitySourceEffect': [('ability', 'ability'), ('duration', 'duration')],
    'ConditionalContinuousEffect': [('effect', 'effect'), ('condition', 'condition'), (None, 'string')],
    'SetBasePowerToughnessSourceEffect': [('count', 'count')],
    'ExileTargetIfDiesEffect': [],
    'TapSourceUnlessPaysEffect': [('life', 'life_cost')],
}

COUNTS = {'CreaturesYouControlCount.PLURAL': 'creatures_you_control',
          'CreaturesYouControlCount.SINGULAR': 'creatures_you_control'}

# classes a card file declares for itself, written by hand as the engine classes that do the same
CUSTOM = {
    # create a 2/2 Robot token tapped and attacking; sacrifice it at end of combat
    'KavaronHarrierEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('RobotToken'), 'amount': 1,
                                     'tapped_and_attacking': True, 'sacrifice_at_end_of_combat': True},
    # whenever one or more opponents are dealt noncombat damage, creatures you control get +1/+0 until end of turn
    'MasterOfBarbsTriggeredAbility': lambda: {
        'class': 'DamagedOpponentsNoncombatTriggeredAbility', 'java': 'MasterOfBarbsTriggeredAbility',
        'effects': [{'class': 'BoostControlledEffect', 'power': 1, 'toughness': 0, 'duration': 'EndOfTurn',
                     'filter': 'creatures', 'exclude_source': False}]},
}

# targets with no arguments
TARGETS = {'TargetAnyTarget', 'TargetCreaturePermanent', 'TargetPlayer', 'TargetOpponent', 'TargetSpell',
           'TargetControlledCreaturePermanent', 'TargetCreatureOrPlaneswalker', 'TargetPlayerOrPlaneswalker'}

# abilities with arguments: class -> (field name, kind) per argument; optional trailing arguments may be left out
ABILITIES = {
    'EntersBattlefieldTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'AttacksTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'SimpleStaticAbility': [('effect', 'effect')],
    'AsEntersBattlefieldAbility': [('effect', 'effect'), (None, 'string')],
    'EntersBattlefieldTappedAbility': [],
    'KickerAbility': [('cost', 'cost_text')],
    # enters with an effect applied, if the condition holds (a replacement effect, not a trigger)
    'EntersBattlefieldAbility': [('effect', 'effect'), ('condition', 'condition'), (None, 'string'), (None, 'string')],
    'EntersBattlefieldAllTriggeredAbility': [('effect', 'effect'), ('filter', 'filter'), ('optional', 'bool')],
    'LandfallAbility': [('effect', 'effect'), ('optional', 'bool')],
    'DiesSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'ActivateAsSorceryActivatedAbility': [('effect', 'effect'), ('cost', 'cost')],
    'SimpleActivatedAbility': [('effect', 'effect'), ('cost', 'cost')],
    'ProwessAbility': [],
    'BeginningOfEndStepTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                           ('optional', 'bool'), ('condition', 'condition')],
}

TARGET_CONTROLLERS = {'TargetController.YOU': 'you', 'TargetController.NEXT': 'any', 'TargetController.ANY': 'any',
                      'TargetController.OPPONENT': 'opponent'}

# arguments a constructor overload leaves out, and what Java uses then
DEFAULTS = {'amount': 1, 'filter': 'creatures', 'exclude_source': False, 'condition': None}

COUNTERS = {'CounterType.P1P1': '+1/+1'}
CONDITIONS = {'KickedCondition.ONCE': 'kicked', 'RaidCondition.instance': 'raid', 'MyTurnCondition.instance': 'my_turn'}
TOKENS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'game', 'permanent', 'token')

DURATIONS = {'Duration.EndOfTurn': 'EndOfTurn', 'Duration.WhileOnBattlefield': 'WhileOnBattlefield'}
FILTERS = {
    'StaticFilters.FILTER_PERMANENT_CREATURES': 'creatures',
    'StaticFilters.FILTER_PERMANENT_CREATURE': 'creatures',
    'StaticFilters.FILTER_EACH_CONTROLLED_CREATURE_P1P1': 'controlled_creatures_with_p1p1',
    'StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL': 'another_creature_you_control',
    'StaticFilters.FILTER_PERMANENTS': 'permanents',
}

# abilities every card has, which the constructor doesn't add
IMPLICIT = {'SpellAbility', 'PlayLandAbility'}


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


def split_args(text):
    out, depth, start, quote = [], 0, 0, False
    for i, c in enumerate(text):
        if c == '"':
            quote = not quote
        elif quote:
            continue
        elif c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ',' and depth == 0:
            out.append(text[start:i])
            start = i + 1
    out.append(text[start:])
    return [a.strip() for a in out] if text.strip() else []


class Translator:
    """Reads one constructor. Local variables (`Effect effect = new X(…)`) hold what they were given."""

    def __init__(self, filters=None):
        self.vars = {}
        # the card class's own filter fields, by name
        self.filters = filters or {}

    def new_expr(self, expr):
        """`new X(args)`, `X.getInstance()` or a local variable -> (X, [args]) or the variable's object"""
        expr = expr.strip()
        # chained setters that only change texts
        while (m := re.fullmatch(r'(.*\))\s*\.(?:setText|setAbilityWord|withFlavorWord|addHint)\((?:"[^"]*"|AbilityWord\.\w+|\w+Hint\.instance)\)', expr, re.S)):
            expr = m.group(1).strip()
        if expr in self.vars:
            return self.vars[expr]
        m = re.fullmatch(r'new (\w+)\(\)', expr)
        if m and m.group(1) in CUSTOM:
            return CUSTOM[m.group(1)]()
        m = re.fullmatch(r'(\w+)\.getInstance\(\)', expr)
        if m:
            return m.group(1), []
        m = re.fullmatch(r'new (\w+)(?:<>)?\((.*)\)', expr)
        if not m:
            raise Unsupported('expression ' + expr)
        return m.group(1), split_args(m.group(2))

    def value(self, kind, arg):
        if kind == 'int':
            if re.fullmatch(r'-?\d+', arg):
                return int(arg)
        elif kind == 'bool':
            if arg in ('true', 'false'):
                return arg == 'true'
        elif kind == 'duration':
            if arg in DURATIONS:
                return DURATIONS[arg]
        elif kind == 'filter':
            if arg in FILTERS:
                return FILTERS[arg]
            if arg in self.filters:
                return self.filters[arg]
        elif kind == 'string':
            if re.fullmatch(r'"[^"]*"', arg):
                return None
        elif kind == 'effect':
            return self.effect(arg)
        elif kind == 'ability':
            return self.ability(arg)
        elif kind == 'count':
            if arg in COUNTS:
                return COUNTS[arg]
        elif kind == 'life_cost':
            m = re.fullmatch(r'new PayLifeCost\((\d+)\)', arg)
            if m:
                return int(m.group(1))
        elif kind == 'target_controller':
            if arg in TARGET_CONTROLLERS:
                return TARGET_CONTROLLERS[arg]
        elif kind == 'condition':
            if arg in CONDITIONS:
                return CONDITIONS[arg]
        elif kind == 'cost_text':
            if re.fullmatch(r'"(\{[^"]*\})"', arg):
                return arg[1:-1]
        elif kind == 'cost':
            return [self.cost(arg)]
        elif kind == 'counter':
            m = re.fullmatch(r'(CounterType\.\w+)\.createInstance\((\d*)\)', arg)
            if m and m.group(1) in COUNTERS:
                return {'name': COUNTERS[m.group(1)], 'amount': int(m.group(2) or 1)}
        elif kind == 'token':
            m = re.fullmatch(r'new (\w+Token)\(\)', arg)
            if m:
                return token(m.group(1))
        raise Unsupported('argument ' + arg)

    def cost(self, expr):
        cls, args = self.new_expr(expr)
        if cls == 'ManaCostsImpl' and len(args) == 1 and re.fullmatch(r'"[^"]*"', args[0]):
            return {'class': 'ManaCost', 'cost': args[0][1:-1]}
        if cls == 'GenericManaCost' and len(args) == 1 and re.fullmatch(r'\d+', args[0]):
            return {'class': 'ManaCost', 'cost': '{' + args[0] + '}'}
        if cls in ('TapSourceCost', 'SacrificeSourceCost') and not args:
            return {'class': cls}
        raise Unsupported('cost ' + expr)

    def build(self, cls, args, spec, what):
        if len(args) > len(spec) or any(k not in ('bool', 'string') and n not in DEFAULTS
                                        for n, k in spec[len(args):]):
            raise Unsupported(what + ' ' + cls + '(' + ','.join(args) + ')')
        out = {'class': cls}
        for name, _ in spec[len(args):]:
            if name in DEFAULTS:
                out[name] = DEFAULTS[name]
        for (name, kind), arg in zip(spec, args):
            v = self.value(kind, arg)
            if name:
                out[name] = v
        return out

    def effect(self, expr):
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls == 'DamageTargetEffect' and args == ['SourcePermanentPowerValue.NOT_NEGATIVE']:
            return {'class': 'DamageTargetSourcePowerEffect'}
        if cls not in EFFECTS:
            raise Unsupported('effect ' + cls)
        return self.build(cls, args, EFFECTS[cls], 'effect')

    def target(self, expr):
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls == 'TargetPermanent' and len(args) == 1 and args[0] in self.filters:
            return {'class': 'TargetPermanent', 'filter': self.filters[args[0]]}
        if cls not in TARGETS or args:
            raise Unsupported('target ' + expr)
        return {'class': cls}

    def ability(self, expr):
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls in KEYWORDS and not args:
            return {'class': cls}
        # `new MenaceAbility(false)`: the flag only changes the reminder text
        if cls == 'MenaceAbility' and args in (['false'], ['true']):
            return {'class': cls}
        if cls == 'SimpleStaticAbility' and len(args) == 2 and args[0] in ('Zone.ALL', 'Zone.BATTLEFIELD'):
            args = args[1:]
        if cls not in ABILITIES:
            raise Unsupported('ability ' + cls)
        a = self.build(cls, args, ABILITIES[cls], 'ability')
        if 'effect' in a:
            a['effects'] = [a.pop('effect')]
        return a

    def assign(self, var, expr):
        """`Effect effect = new X(…)`: what kind of thing the right side is decides how it's read"""
        cls = self.new_expr(expr)[0]
        if cls == 'Mode':
            args = self.new_expr(expr)[1]
            self.vars[var] = {'class': 'Mode', 'effects': [self.effect(a) for a in args], 'targets': []}
        elif cls in EFFECTS:
            self.vars[var] = self.effect(expr)
        elif cls in TARGETS:
            self.vars[var] = self.target(expr)
        else:
            self.vars[var] = self.ability(expr)


def translate(card_name, subtypes, facts):
    if card_name not in facts:
        raise Unsupported('no card facts')
    java_abilities = facts[card_name]
    if card_name in BASIC_LANDS:
        return {'name': card_name, 'types': ['LAND'], 'supertypes': ['BASIC'], 'subtypes': [card_name],
                'colors': [], 'abilities': [{'class': 'PlayLandAbility'}, {'class': BASIC_LANDS[card_name]}]}
    cls = class_name(card_name)
    path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no card class ' + cls)
    source = open(path).read()
    if re.search(r'extends (?!CardImpl\b)\w+', re.search(r'public final class \w+ extends \w+', source).group(0)):
        raise Unsupported('card class is not a CardImpl')
    t = Translator(card_filters(source))
    card = {'name': card_name}
    added, spell, modes = [], {'effects': [], 'targets': []}, []
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},(".*?"|null)\)', st)
        if m:
            card['types'] = [x.strip().split('.')[-1] for x in m.group(1).split(',') if x.strip()]
            card['cost'] = '' if m.group(2) == 'null' else m.group(2)[1:-1]
            continue
        m = re.fullmatch(r'this\.subtype\.add\((.*)\)', st)
        if m:
            for x in split_args(m.group(1)):
                key = x.split('.')[-1]
                if key not in subtypes:
                    raise Unsupported('subtype ' + x)
                card.setdefault('subtypes', []).append(subtypes[key])
            continue
        m = re.fullmatch(r'this\.supertype\.add\((.*)\)', st)
        if m:
            card.setdefault('supertypes', []).extend(x.split('.')[-1] for x in split_args(m.group(1)))
            continue
        m = re.fullmatch(r'this\.(power|toughness)=new MageInt\((-?\d+)\)', st)
        if m:
            card[m.group(1)] = int(m.group(2))
            continue
        m = re.fullmatch(r'(?:final )?\w+ (\w+)=(.*)', st)
        if m:
            t.assign(m.group(1), m.group(2))
            continue
        m = re.fullmatch(r'(\w+)\.set(?:Outcome|Text|TargetPointer)\(.*\)', st)
        if m and m.group(1) in t.vars:
            continue
        m = re.fullmatch(r'(\w+)\.add(Effect|Target|Cost)\((.*)\)', st)
        if m and m.group(1) in t.vars and 'class' in t.vars[m.group(1)]:
            holder = t.vars[m.group(1)]
            if m.group(2) == 'Effect':
                holder.setdefault('effects', []).append(t.effect(m.group(3)))
            elif m.group(2) == 'Target':
                holder.setdefault('targets', []).append(t.target(m.group(3)))
            else:
                holder.setdefault('cost', []).append(t.cost(m.group(3)))
            continue
        m = re.fullmatch(r'this\.addAbility\((.*)\)', st)
        if m:
            args = split_args(m.group(1))
            # `addAbility(ability, new SomeWatcher())`: the watcher is the engine's own bookkeeping
            if len(args) == 2 and re.fullmatch(r'new \w+Watcher\(\)', args[1].strip()):
                args = args[:1]
            added.append(t.ability(','.join(args)))
            continue
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addEffect\((.*)\)', st)
        if m:
            spell['effects'].append(t.effect(m.group(1)))
            continue
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addMode\((.*)\)', st)
        if m:
            arg = m.group(1).strip()
            if arg in t.vars:
                mode = t.vars[arg]
            else:
                cls, args = t.new_expr(arg)
                if cls != 'Mode':
                    raise Unsupported('mode ' + arg)
                mode = {'class': 'Mode', 'effects': [t.effect(a) for a in args], 'targets': []}
            modes.append({'effects': mode.get('effects', []), 'targets': mode.get('targets', [])})
            continue
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addTarget\((.*)\)', st)
        if m:
            spell['targets'].append(t.target(m.group(1)))
            continue
        raise Unsupported('statement ' + st)
    if 'types' not in card:
        raise Unsupported('no card types')
    # the abilities in Java's order (decision logs number them so), each matched to one the constructor added
    abilities = []
    for fact in java_abilities:
        if fact['class'] in IMPLICIT:
            abilities.append({'class': fact['class']})
            continue
        match = next((a for a in added if a.get('java', a['class']) == fact['class']), None)
        if match is None:
            raise Unsupported('ability ' + fact['class'] + ' (added by another ability)')
        added.remove(match)
        if match.pop('java', None) or match['class'] in ABILITIES:
            match['rule'] = fact['rule']
        abilities.append(match)
    if added:
        raise Unsupported('abilities Java does not list: ' + ', '.join(a['class'] for a in added))
    card['abilities'] = abilities
    if modes:
        # "choose one": the spell ability's own effects and targets are its first mode
        card['spell'] = {'modes': [spell] + modes}
    elif spell['effects'] or spell['targets']:
        card['spell'] = spell
    return card


# filter fields cards declare (`private static final FilterPermanent filter = new …; static { filter.add(…); }`),
# by what they're built from
CARD_FILTERS = {
    ('FilterCreatureOrPlaneswalkerPermanent', ('TargetController.NOT_YOU.getControllerPredicate()',)):
        'opponents_creature_or_planeswalker',
    ('FilterControlledCreaturePermanent', ('CounterType.P1P1.getPredicate()',)): 'controlled_creatures_with_p1p1',
}


def card_filters(source):
    out = {}
    for m in re.finditer(r'static final \w+ (\w+)\s*=\s*new (\w+)\((?:"[^"]*")?\);', source):
        name, cls = m.group(1), m.group(2)
        preds = tuple(canonical(p) for p in re.findall(name + r'\.add\((.*?)\);', source))
        if (cls, preds) in CARD_FILTERS:
            out[name] = CARD_FILTERS[(cls, preds)]
    return out


def token(cls):
    """A token class from Mage/…/permanent/token: name, types, colors, subtypes, power/toughness, keyword abilities"""
    path = os.path.join(TOKENS, cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no token class ' + cls)
    source = open(path).read()
    m = re.search(r'public ' + cls + r'\(\)\s*\{(.*?)\n    \}', source, re.S)
    if not m:
        raise Unsupported('token constructor ' + cls)
    out = {'types': [], 'colors': [], 'subtypes': [], 'abilities': []}
    subtypes = subtype_names()
    colors = {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}
    for st in statements(m.group(1)):
        if (n := re.fullmatch(r'super\("([^"]+)",".*"\)', st)):
            out['name'] = n.group(1)
        elif (n := re.fullmatch(r'cardType\.add\(CardType\.(\w+)\)', st)):
            out['types'].append(n.group(1))
        elif (n := re.fullmatch(r'color\.set(\w+)\(true\)', st)):
            out['colors'].append(colors[n.group(1)])
        elif (n := re.fullmatch(r'subtype\.add\(SubType\.(\w+)\)', st)):
            out['subtypes'].append(subtypes[n.group(1)])
        elif (n := re.fullmatch(r'(power|toughness)=new MageInt\((\d+)\)', st)):
            out[n.group(1)] = int(n.group(2))
        elif (n := re.fullmatch(r'addAbility\((\w+)\.getInstance\(\)\)', st)) and n.group(1) in KEYWORDS:
            out['abilities'].append({'class': n.group(1)})
        else:
            raise Unsupported('token statement ' + st)
    return out


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
    parser.add_argument('--facts', default=DEFAULT_FACTS)
    parser.add_argument('--out', default=DEFAULT_OUT)
    parser.add_argument('--report')
    args = parser.parse_args()
    pool = [l.strip() for l in open(args.pool)] if args.pool else school_pool()
    subtypes = subtype_names()
    facts = {c['name']: c['abilities'] for c in json.load(open(args.facts))}
    cards, report = [], []
    for name in pool:
        if not name:
            continue
        try:
            cards.append(translate(name, subtypes, facts))
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

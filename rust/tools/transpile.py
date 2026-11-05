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
    'GainAbilityAllEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter'),
                             ('exclude_source', 'bool')],
    'ConditionalOneShotEffect': [('effect', 'effect'), ('otherwise', 'effect'), ('condition', 'condition'),
                                 (None, 'string')],
    'DamagePlayersEffect': [('amount', 'int'), ('controller', 'target_controller')],
    'SacrificeSourceEffect': [],
    'DoIfCostPaid': [('effect', 'effect'), ('cost', 'cost')],
    'GainAbilityControlledEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter'),
                                    ('exclude_source', 'bool')],
    'GainAbilityTargetEffect': [('ability', 'ability'), ('duration', 'duration')],
    'GainAbilitySourceEffect': [('ability', 'ability'), ('duration', 'duration')],
    'ConditionalContinuousEffect': [('effect', 'effect'), ('condition', 'condition'), (None, 'string')],
    'SetBasePowerToughnessSourceEffect': [('count', 'count')],
    'ExileTargetIfDiesEffect': [],
    'TapSourceUnlessPaysEffect': [('life', 'life_cost')],
    'LoseLifeOpponentsEffect': [('amount', 'int')],
    'LoseLifeSourceControllerEffect': [('amount', 'int')],
    'LoseLifeTargetEffect': [('amount', 'int')],
    'DestroyAllEffect': [('filter', 'filter')],
    'DamageAllEffect': [('amount', 'int'), ('filter', 'filter')],
    'BoostAllEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), ('filter', 'filter'),
                       ('exclude_source', 'bool')],
    'AddCountersAllEffect': [('counter', 'counter'), ('filter', 'filter')],
    'ExileTargetEffect': [],
    'ReturnToHandTargetEffect': [],
    'TapTargetEffect': [],
    'UntapTargetEffect': [],
    'TapSourceEffect': [(None, 'bool')],
    'CounterUnlessPaysEffect': [('cost', 'cost')],
    'DamageControllerEffect': [('amount', 'int')],
    'DrawCardTargetEffect': [('amount', 'int')],
    'ScryEffect': [('amount', 'int'), (None, 'bool')],
    'SurveilEffect': [('amount', 'int'), (None, 'bool')],
    # subtypes aren't in the fingerprint and no rule here reads them yet
    'AddCardSubTypeSourceEffect': [(None, 'duration'), (None, 'bool'), (None, 'subtype')],
}

COUNTS = {'CreaturesYouControlCount.PLURAL': 'creatures_you_control',
          'CreaturesYouControlCount.SINGULAR': 'creatures_you_control',
          'LandsYouControlCount.instance': 'lands_you_control'}
PUT_CARDS = {'HAND', 'GRAVEYARD', 'BATTLEFIELD', 'BATTLEFIELD_TAPPED', 'EXILED', 'TOP_ANY', 'BOTTOM_ANY', 'BOTTOM_RANDOM'}

# classes a card file declares for itself, written by hand as the engine classes that do the same
CUSTOM = {
    # create a 2/2 Robot token tapped and attacking; sacrifice it at end of combat
    'KavaronHarrierEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('RobotToken'), 'amount': 1,
                                     'tapped_and_attacking': True, 'sacrifice_at_end_of_combat': True},
    # whenever another creature you control enters (the "controlled" filter adds "you control")
    'EntersBattlefieldControlledTriggeredAbility': None,
    # whenever one or more opponents are dealt noncombat damage, creatures you control get +1/+0 until end of turn
    'MasterOfBarbsTriggeredAbility': lambda: {
        'class': 'DamagedOpponentsNoncombatTriggeredAbility', 'java': 'MasterOfBarbsTriggeredAbility',
        'effects': [{'class': 'BoostControlledEffect', 'power': 1, 'toughness': 0, 'duration': 'EndOfTurn',
                     'filter': [CREATURE], 'exclude_source': False}]},
}

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
    # the step triggers take a TargetController first; the shorter overloads mean "your" (filled in by `ability`)
    'BeginningOfUpkeepTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                          ('optional', 'bool')],
    'BeginningOfCombatTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                          ('optional', 'bool')],
    'BeginningOfEndStepTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                           ('optional', 'bool'), ('condition', 'condition')],
    'GainLifeControllerTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), (None, 'bool'),
                                           (None, 'false')],
    'DealsCombatDamageToAPlayerTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'),
                                                   ('set_target_pointer', 'bool')],
    'EntersBattlefieldOrDiesSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), (None, 'bool')],
    'EntersBattlefieldOrAttacksSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'DiesThisOrAnotherTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), ('filter', 'filter')],
    'SpellCastControllerTriggeredAbility': [('effect', 'effect'), ('filter', 'filter'), ('optional', 'bool'),
                                            (None, 'no_pointer')],
}

STEP_TRIGGERS = {'BeginningOfUpkeepTriggeredAbility': 'upkeep', 'BeginningOfCombatTriggeredAbility': 'combat',
                 'BeginningOfEndStepTriggeredAbility': 'end'}


def triggered(a):
    """A Java triggered-ability class, as read from its constructor, as the engine's one `TriggeredAbility`: what it
    triggers on, then the rest"""
    cls = a['class']
    if cls in STEP_TRIGGERS:
        on = {'e': 'step_begins', 'step': STEP_TRIGGERS[cls], 'who': a.pop('controller')}
    elif cls == 'ProwessAbility':
        on = {'e': 'spell_cast', 'filter': [{'p': 'not', 'x': CREATURE}], 'who': 'you'}
        a['effects'] = [{'class': 'BoostSourceEffect', 'power': 1, 'toughness': 1, 'duration': 'EndOfTurn'}]
    elif cls == 'LandfallAbility':
        on = {'e': 'enters_all', 'filter': [LAND, YOU]}
    elif cls == 'EntersBattlefieldAllTriggeredAbility':
        on = {'e': 'enters_all', 'filter': a.pop('filter')}
    elif cls == 'DiesThisOrAnotherTriggeredAbility':
        on = {'e': 'dies_all', 'filter': a.pop('filter', [CREATURE]), 'include_self': True}
    elif cls == 'SpellCastControllerTriggeredAbility':
        on = {'e': 'spell_cast', 'filter': a.pop('filter'), 'who': 'you'}
    elif cls in SIMPLE_TRIGGERS:
        on = {'e': SIMPLE_TRIGGERS[cls]}
    else:
        return a
    out = {'class': 'TriggeredAbility', 'java': a.pop('java', cls), 'on': on}
    out.update((k, v) for k, v in a.items() if k != 'class')
    return out


SIMPLE_TRIGGERS = {'EntersBattlefieldTriggeredAbility': 'enters', 'AttacksTriggeredAbility': 'attacks',
                   'DiesSourceTriggeredAbility': 'dies', 'EntersBattlefieldOrDiesSourceTriggeredAbility':
                   'enters_or_dies', 'EntersBattlefieldOrAttacksSourceTriggeredAbility': 'enters_or_attacks',
                   'GainLifeControllerTriggeredAbility': 'you_gain_life',
                   'DealsCombatDamageToAPlayerTriggeredAbility': 'combat_damage_to_player',
                   'DamagedOpponentsNoncombatTriggeredAbility': 'opponents_dealt_noncombat_damage'}

TARGET_CONTROLLERS = {'TargetController.YOU': 'you', 'TargetController.NEXT': 'any', 'TargetController.ANY': 'any',
                      'TargetController.OPPONENT': 'opponent'}

COUNTERS = {'CounterType.P1P1': '+1/+1'}
CONDITIONS = {'KickedCondition.ONCE': {'c': 'kicked'}, 'RaidCondition.instance': {'c': 'raid'},
              'MyTurnCondition.instance': {'c': 'my_turn'}}
TOKENS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'game', 'permanent', 'token')

DURATIONS = {'Duration.EndOfTurn': 'EndOfTurn', 'Duration.WhileOnBattlefield': 'WhileOnBattlefield'}

STATIC_FILTERS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'filter', 'StaticFilters.java')


def type_pred(t):
    return {'p': 'type', 't': t}


CREATURE, LAND, YOU, OPPONENT = type_pred('CREATURE'), type_pred('LAND'), {'p': 'controller', 'who': 'you'}, \
    {'p': 'controller', 'who': 'opponent'}

# arguments a constructor overload leaves out, and what Java uses then
DEFAULTS = {'amount': 1, 'filter': [CREATURE], 'exclude_source': False, 'condition': None, 'duration': 'EndOfTurn'}

# filter classes: the predicates their constructors add (a SubType argument adds that subtype too)
FILTER_CLASSES = {
    'FilterPermanent': [], 'FilterCreaturePermanent': [CREATURE], 'FilterControlledPermanent': [YOU],
    'FilterControlledCreaturePermanent': [CREATURE, YOU], 'FilterLandPermanent': [LAND],
    'FilterControlledLandPermanent': [LAND, YOU], 'FilterNonlandPermanent': [{'p': 'not', 'x': LAND}],
    'FilterArtifactPermanent': [type_pred('ARTIFACT')], 'FilterControlledArtifactPermanent': [type_pred('ARTIFACT'), YOU],
    'FilterEnchantmentPermanent': [type_pred('ENCHANTMENT')],
    'FilterControlledEnchantmentPermanent': [type_pred('ENCHANTMENT'), YOU],
    'FilterPlaneswalkerPermanent': [type_pred('PLANESWALKER')],
    'FilterControlledPlaneswalkerPermanent': [type_pred('PLANESWALKER'), YOU],
    'FilterCreatureOrPlaneswalkerPermanent': [{'p': 'or', 'xs': [CREATURE, type_pred('PLANESWALKER')]}],
    'FilterOpponentsCreaturePermanent': [CREATURE, OPPONENT],
    'FilterSpell': [], 'FilterCreatureSpell': [CREATURE], 'FilterNoncreatureSpell': [{'p': 'not', 'x': CREATURE}],
    'FilterInstantOrSorcerySpell': [{'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}],
    'FilterCard': [], 'FilterCreatureCard': [CREATURE], 'FilterLandCard': [LAND],
    'FilterNonlandCard': [{'p': 'not', 'x': LAND}], 'FilterPermanentCard': [{'p': 'permanent_card'}],
    'FilterInstantOrSorceryCard': [{'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}],
    'FilterBasicLandCard': [LAND, {'p': 'supertype', 's': 'BASIC'}],
    'FilterStackObject': [], 'FilterCreatureCards': [CREATURE],
}

COMPARISONS = {'FEWER_THAN', 'MORE_THAN', 'EQUAL_TO', 'OR_LESS', 'OR_GREATER'}
COLORS = {'WHITE': 'W', 'BLUE': 'U', 'BLACK': 'B', 'RED': 'R', 'GREEN': 'G'}
KEYWORD_CLASSES = {k: k for k in KEYWORDS}


def predicate(text, subtypes):
    """A Java predicate expression (canonical text) -> a predicate object"""
    text = text.strip()
    while text.startswith('(') and text.endswith(')') and split_args(text[1:-1]) == [text[1:-1].strip()]:
        text = text[1:-1].strip()
    if (m := re.fullmatch(r'CardType\.(\w+)\.getPredicate\(\)', text)):
        return type_pred(m.group(1))
    if (m := re.fullmatch(r'SubType\.(\w+)\.getPredicate\(\)', text)) and m.group(1) in subtypes:
        return {'p': 'subtype', 's': subtypes[m.group(1)]}
    if (m := re.fullmatch(r'SuperType\.(\w+)\.getPredicate\(\)', text)):
        return {'p': 'supertype', 's': m.group(1)}
    if (m := re.fullmatch(r'TargetController\.(YOU|OPPONENT|NOT_YOU)\.get(Controller|Owner)Predicate\(\)', text)):
        return {'p': m.group(2).lower(), 'who': 'you' if m.group(1) == 'YOU' else 'opponent'}
    if text == 'AnotherPredicate.instance':
        return {'p': 'another'}
    if (m := re.fullmatch(r'new (ManaValue|Power|Toughness)Predicate\(ComparisonType\.(\w+),(-?\d+)\)', text)) \
            and m.group(2) in COMPARISONS:
        return {'p': {'ManaValue': 'mv'}.get(m.group(1), m.group(1).lower()), 'cmp': m.group(2), 'n': int(m.group(3))}
    if (m := re.fullmatch(r'new ColorPredicate\(ObjectColor\.(\w+)\)', text)) and m.group(1) in COLORS:
        return {'p': 'color', 'c': COLORS[m.group(1)]}
    if (m := re.fullmatch(r'TappedPredicate\.(TAPPED|UNTAPPED)', text)):
        return {'p': 'tapped', 'v': m.group(1) == 'TAPPED'}
    if text == 'AttackingPredicate.instance':
        return {'p': 'attacking'}
    if text == 'AttackingOrBlockingPredicate.instance':
        return {'p': 'in_combat'}
    if (m := re.fullmatch(r'TokenPredicate\.(TRUE|FALSE)', text)):
        return {'p': 'token', 'v': m.group(1) == 'TRUE'}
    if (m := re.fullmatch(r'(CounterType\.\w+)\.getPredicate\(\)', text)) and m.group(1) in COUNTERS:
        return {'p': 'counter', 'name': COUNTERS[m.group(1)]}
    if (m := re.fullmatch(r'new AbilityPredicate\((\w+)\.class\)', text)) and m.group(1) in KEYWORDS:
        return {'p': 'ability', 'a': {'class': m.group(1)}}
    if (m := re.fullmatch(r'new NamePredicate\("([^"]*)"\)', text)):
        return {'p': 'name', 'n': m.group(1)}
    if (m := re.fullmatch(r'Predicates\.not\((.*)\)', text)):
        return {'p': 'not', 'x': predicate(m.group(1), subtypes)}
    if (m := re.fullmatch(r'Predicates\.(or|and)\((.*)\)', text)):
        return {'p': m.group(1), 'xs': [predicate(x, subtypes) for x in split_args(m.group(2))]}
    raise Unsupported('predicate ' + text)


def filter_object(cls, args, adds, subtypes):
    """`new <cls>(args)` plus the predicates later `add`ed to it -> a filter (a list of predicates)"""
    if cls not in FILTER_CLASSES:
        raise Unsupported('filter class ' + cls)
    preds = [dict(p) for p in FILTER_CLASSES[cls]]
    for a in args:
        a = a.strip()
        if re.fullmatch(r'"[^"]*"', a):
            continue
        if (m := re.fullmatch(r'SubType\.(\w+)', a)) and m.group(1) in subtypes:
            preds.append({'p': 'subtype', 's': subtypes[m.group(1)]})
            continue
        raise Unsupported('filter argument ' + a)
    for x in adds:
        preds.append(predicate(x, subtypes))
    return preds


def declared_filters(source, subtypes):
    """filter fields (`static final FilterX name = new FilterX(…); static { name.add(…); }`) -> filters by name; one
    that can't be read is left out (the card fails only if it uses it)"""
    out = {}
    for m in re.finditer(r'static final (?:\w+) (\w+)\s*=\s*new (Filter\w+)\((.*?)\);', source, re.S):
        name, cls, args = m.group(1), m.group(2), split_args(canonical(m.group(3)))
        adds = [canonical(p) for p in re.findall(r'\b' + name + r'\.add\((.*?)\);', source, re.S)]
        try:
            out[name] = filter_object(cls, args, adds, subtypes)
        except Unsupported as e:
            out[name] = e
    return out


_static_filters = None


def static_filters(subtypes):
    global _static_filters
    if _static_filters is None:
        _static_filters = declared_filters(open(STATIC_FILTERS).read(), subtypes)
    return _static_filters


# targets: class -> (what it targets, the filter its constructors use when none is given)
TARGET_CLASSES = {
    'TargetPermanent': ('TargetPermanent', []),
    'TargetCreaturePermanent': ('TargetPermanent', [CREATURE]),
    'TargetControlledCreaturePermanent': ('TargetPermanent', [CREATURE, YOU]),
    'TargetControlledPermanent': ('TargetPermanent', [YOU]),
    'TargetOpponentsCreaturePermanent': ('TargetPermanent', [CREATURE, OPPONENT]),
    'TargetNonlandPermanent': ('TargetPermanent', [{'p': 'not', 'x': LAND}]),
    'TargetArtifactPermanent': ('TargetPermanent', [type_pred('ARTIFACT')]),
    'TargetEnchantmentPermanent': ('TargetPermanent', [type_pred('ENCHANTMENT')]),
    'TargetLandPermanent': ('TargetPermanent', [LAND]),
    'TargetControlledLandPermanent': ('TargetPermanent', [LAND, YOU]),
    'TargetPlaneswalkerPermanent': ('TargetPermanent', [type_pred('PLANESWALKER')]),
    'TargetCreatureOrPlaneswalker': ('TargetPermanent', [{'p': 'or', 'xs': [CREATURE, type_pred('PLANESWALKER')]}]),
    'TargetAttackingCreature': ('TargetPermanent', [CREATURE, {'p': 'attacking'}]),
    'TargetAttackingOrBlockingCreature': ('TargetPermanent', [CREATURE, {'p': 'in_combat'}]),
    'TargetSpell': ('TargetSpell', []),
    'TargetCardInGraveyard': ('TargetCardInGraveyard', []),
    'TargetCardInYourGraveyard': ('TargetCardInGraveyard', [{'p': 'owner', 'who': 'you'}]),
    'TargetCardInOpponentsGraveyard': ('TargetCardInGraveyard', [{'p': 'owner', 'who': 'opponent'}]),
}
# targets of players (no arguments)
PLAYER_TARGETS = {'TargetAnyTarget', 'TargetPlayer', 'TargetOpponent', 'TargetPlayerOrPlaneswalker'}

# chained calls that only change texts, hints or the AI's view
TEXT_SETTERS = {'setText', 'setAbilityWord', 'withFlavorWord', 'addHint', 'concatBy', 'setTriggerPhrase', 'withChooseHint',
                'setOutcome'}

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

    def __init__(self, filters=None, conditions=None, subtypes=None):
        self.vars = {}
        # the card class's own filter and condition fields, by name
        self.filters = filters or {}
        self.conditions = conditions or {}
        self.subtypes = subtypes or {}

    def condition(self, arg):
        """a condition argument: a known instance, a field of the card, or `new XCondition(…)`"""
        arg = canonical(arg)
        if arg in CONDITIONS:
            return CONDITIONS[arg]
        if arg in self.conditions:
            return self.condition(self.conditions[arg])
        if arg == 'YouControlABasicLandCondition.instance':
            return {'c': 'permanents', 'filter': [LAND, {'p': 'supertype', 's': 'BASIC'}], 'cmp': 'OR_GREATER', 'n': 1}
        if (m := re.fullmatch(r'new (YouControlPermanentCondition|PermanentsOnTheBattlefieldCondition)\((.*)\)', arg)):
            args = split_args(m.group(2))
            out = {'c': 'permanents', 'filter': self.filter(args[0]), 'cmp': 'OR_GREATER', 'n': 1}
            if len(args) >= 3 and (c := re.fullmatch(r'ComparisonType\.(\w+)', args[1])) and c.group(1) in COMPARISONS \
                    and re.fullmatch(r'\d+', args[2]):
                out['cmp'], out['n'] = c.group(1), int(args[2])
            elif len(args) != 1:
                raise Unsupported('condition ' + arg)
            if m.group(1) == 'PermanentsOnTheBattlefieldCondition' and not (len(args) == 4 and args[3] == 'true'):
                out['all'] = True
            return out
        if (m := re.fullmatch(r'new SourceHasCounterCondition\((CounterType\.\w+),(\d+)\)', arg)) and m.group(1) in COUNTERS:
            return {'c': 'source_counter', 'name': COUNTERS[m.group(1)], 'n': int(m.group(2))}
        if (m := re.fullmatch(r'new InvertCondition\((.*)\)', arg)):
            return {'c': 'not', 'x': self.condition(m.group(1))}
        raise Unsupported('condition ' + arg)

    def filter(self, arg):
        """a filter argument: a field of the card, a `StaticFilters` one, or `new FilterX(…)`"""
        arg = arg.strip()
        name = arg[len('StaticFilters.'):] if arg.startswith('StaticFilters.') else arg
        found = self.filters.get(name)
        if found is None and re.fullmatch(r'FILTER_\w+', name):
            found = static_filters(self.subtypes).get(name)
        if found is None and (m := re.fullmatch(r'new (Filter\w+)\((.*)\)', arg, re.S)):
            found = filter_object(m.group(1), split_args(m.group(2)), [], self.subtypes)
        if isinstance(found, Unsupported):
            raise found
        if found is None:
            raise Unsupported('filter ' + arg)
        return found

    @staticmethod
    def strip_setters(expr):
        """chained setters that only change texts or hints: `….setText("…")`, `….addHint(x.getHint())`"""
        expr = expr.strip()
        while expr.endswith(')'):
            depth, i = 0, len(expr) - 1
            while i >= 0:
                depth += {')': 1, '(': -1}.get(expr[i], 0)
                if depth == 0:
                    break
                i -= 1
            m = re.search(r'\.(\w+)$', expr[:i])
            if not m or m.group(1) not in TEXT_SETTERS:
                break
            expr = expr[:m.start()].strip()
        return expr

    def new_expr(self, expr):
        """`new X(args)`, `X.getInstance()` or a local variable -> (X, [args]) or the variable's object"""
        expr = self.strip_setters(expr)
        if expr in self.vars:
            return self.vars[expr]
        m = re.fullmatch(r'new (\w+)\(\)', expr)
        if m and m.group(1) in CUSTOM and CUSTOM[m.group(1)]:
            return triggered(CUSTOM[m.group(1)]())
        # whenever a permanent of the filter you control enters
        m = re.fullmatch(r'new EntersBattlefieldControlledTriggeredAbility\((.*)\)', expr, re.S)
        if m:
            args = split_args(m.group(1))
            if args[0].strip() == 'Zone.BATTLEFIELD':
                args = args[1:]
            pointer = False
            if args and args[-1].strip().startswith('SetTargetPointer.'):
                if args[-1].strip() not in ('SetTargetPointer.PERMANENT', 'SetTargetPointer.NONE'):
                    raise Unsupported('EntersBattlefieldControlledTriggeredAbility(' + m.group(1) + ')')
                pointer = args[-1].strip() == 'SetTargetPointer.PERMANENT'
                args = args[:-1]
            if len(args) not in (2, 3) or (len(args) == 3 and args[2].strip() not in ('true', 'false')):
                raise Unsupported('EntersBattlefieldControlledTriggeredAbility(' + m.group(1) + ')')
            return triggered({'class': 'EntersBattlefieldAllTriggeredAbility',
                              'java': 'EntersBattlefieldControlledTriggeredAbility',
                              'effects': [self.effect(args[0])], 'filter': self.filter(args[1]) + [YOU],
                              'optional': len(args) == 3 and args[2].strip() == 'true',
                              'set_target_pointer': pointer})
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
        elif kind == 'false':
            if arg == 'false':
                return False
        elif kind == 'no_pointer':
            if arg == 'SetTargetPointer.NONE':
                return None
        elif kind == 'duration':
            if arg in DURATIONS:
                return DURATIONS[arg]
        elif kind == 'filter':
            return self.filter(arg)
        elif kind == 'string':
            if re.fullmatch(r'"[^"]*"(?:\s*\+\s*"[^"]*")*', arg):
                return None
        elif kind == 'effect':
            return None if arg == 'null' else self.effect(arg)
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
            return self.condition(arg)
        elif kind == 'zone':
            if arg in ('Zone.BATTLEFIELD',):
                return None
        elif kind == 'subtype':
            if re.fullmatch(r'SubType\.\w+', arg):
                return None
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
        if len(args) > len(spec) or any(k not in ('bool', 'string', 'false', 'no_pointer') and n not in DEFAULTS
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
        if cls == 'LookLibraryAndPickControllerEffect':
            return self.look_library(args)
        if cls not in EFFECTS:
            raise Unsupported('effect ' + cls)
        return self.build(cls, args, EFFECTS[cls], 'effect')

    def look_library(self, args):
        """`LookLibraryAndPickControllerEffect`'s overloads: (n, pick, picked, looked[, upTo]) takes any cards;
        (n, pick, filter, picked, looked[, optional]) asks first when optional (the default) and is "up to" for more
        than one"""
        args = [a.strip() for a in args]
        out = {'class': 'LookLibraryAndPickControllerEffect'}
        if re.fullmatch(r'\d+', args[0]):
            out['look'] = int(args[0])
        elif args[0] in COUNTS:
            out['look_count'] = COUNTS[args[0]]
        else:
            raise Unsupported('argument ' + args[0])
        if not re.fullmatch(r'\d+', args[1]):
            raise Unsupported('argument ' + args[1])
        out['pick'] = int(args[1])
        rest = args[2:]
        puts = [a for a in rest if a.startswith('PutCards.')]
        if len(puts) != 2 or any(p[len('PutCards.'):] not in PUT_CARDS for p in puts):
            raise Unsupported('LookLibraryAndPickControllerEffect(' + ','.join(args) + ')')
        out['picked'], out['looked'] = (p[len('PutCards.'):] for p in puts)
        if rest[0].startswith('PutCards.'):
            # any cards; "up to" when asked
            out['up_to'] = len(rest) == 3 and rest[2] == 'true'
            if len(rest) not in (2, 3):
                raise Unsupported('LookLibraryAndPickControllerEffect(' + ','.join(args) + ')')
        else:
            out['filter'] = self.filter(rest[0])
            out['optional'] = not (len(rest) == 4 and rest[3] == 'false')
            out['up_to'] = out['pick'] > 1
        return out

    def target(self, expr):
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls in PLAYER_TARGETS and not args:
            return {'class': cls}
        if cls not in TARGET_CLASSES:
            raise Unsupported('target ' + expr)
        kind, preds = TARGET_CLASSES[cls]
        out = {'class': kind, 'filter': [dict(p) for p in preds]}
        ints = [int(a) for a in args if re.fullmatch(r'\d+', a.strip())]
        rest = [a for a in args if not re.fullmatch(r'\d+', a.strip())]
        if rest and rest[-1].strip() == 'false':
            rest = rest[:-1]
        if len(rest) > 1:
            raise Unsupported('target ' + expr)
        if rest:
            out['filter'] = self.filter(rest[0])
        if kind == 'TargetSpell':
            if ints:
                raise Unsupported('target ' + expr)
        elif len(ints) == 1:
            out['min'] = out['max'] = ints[0]
        elif len(ints) == 2:
            out['min'], out['max'] = ints
        elif ints:
            raise Unsupported('target ' + expr)
        return out

    def ability(self, expr):
        # `….withInterveningIf(condition)`: the "if" of a triggered ability
        m = re.fullmatch(r'(.*\))\s*\.withInterveningIf\((\w+(?:\.instance)?)\)', self.strip_setters(expr), re.S)
        if m:
            a = self.ability(m.group(1))
            a['condition'] = self.value('condition', m.group(2))
            return a
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
        if cls == 'EntersBattlefieldTappedUnlessAbility' and len(args) in (1, 2):
            return {'class': 'EntersBattlefieldAbility', 'java': cls, 'effects': [
                {'class': 'ConditionalOneShotEffect', 'otherwise': {'class': 'TapSourceEffect'},
                 'condition': self.condition(args[0])}]}
        if cls in STEP_TRIGGERS and args and not args[0].strip().startswith('TargetController.'):
            args = ['TargetController.YOU'] + args
        if cls == 'SpellCastControllerTriggeredAbility' and len(args) == 2:
            args = [args[0], 'StaticFilters.FILTER_SPELL_A', args[1]]
        if cls not in ABILITIES:
            raise Unsupported('ability ' + cls)
        a = self.build(cls, args, ABILITIES[cls], 'ability')
        if 'effect' in a:
            a['effects'] = [a.pop('effect')]
        return triggered(a)

    def assign(self, var, expr):
        """`Effect effect = new X(…)`: what kind of thing the right side is decides how it's read"""
        if re.search(r'\.withInterveningIf\(\w+(?:\.instance)?\)\s*$', self.strip_setters(expr)):
            self.vars[var] = self.ability(expr)
            return
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            self.vars[var] = obj
            return
        cls = obj[0]
        if cls == 'Mode':
            args = self.new_expr(expr)[1]
            self.vars[var] = {'class': 'Mode', 'effects': [self.effect(a) for a in args], 'targets': []}
        elif cls in EFFECTS:
            self.vars[var] = self.effect(expr)
        elif cls in TARGET_CLASSES or cls in PLAYER_TARGETS:
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
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes)
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
        st = re.sub(r'^getSpellAbility\(\)', 'this.getSpellAbility()', st)
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
        # an effect's outcome only steers the Java AI
        if re.fullmatch(r'\w+\.setOutcome\(Outcome\.\w+\)', st):
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
    rule = next((f['rule'] for f in java_abilities if f['class'] == 'SpellAbility'), '')
    if modes:
        # "choose one": the spell ability's own effects and targets are its first mode
        card['spell'] = {'modes': [spell] + modes, 'rule': rule}
    elif spell['effects'] or spell['targets']:
        card['spell'] = dict(spell, rule=rule)
    return card


def card_conditions(source):
    """condition fields (`private static final Condition condition = new …;`) -> their texts, by name"""
    return {m.group(1): canonical(m.group(2))
            for m in re.finditer(r'static final \w*Condition (\w+)\s*=\s*(.*?);', source, re.S)}


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

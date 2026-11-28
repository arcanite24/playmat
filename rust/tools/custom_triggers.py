"""A card's own triggered ability classes (`class XTriggeredAbility extends TriggeredAbilityImpl` in the card's file),
read conservatively from their constructor, `checkEventType`, and `checkTrigger` bodies.

Only clean, unambiguous patterns with no hidden side effects (no watchers, no dynamic target pointers, no state
mutations) are translated; anything else raises Unsupported so the card stays out of cards.json until supported.
"""
import re

import custom_effects as CE
import custom_statics as CS

Unsupported = CE.Unsupported


def translate(t, cls, args, source, transpiler):
    """a card's own TriggeredAbilityImpl class -> an engine TriggeredAbility dict, or Unsupported"""
    CE.T = transpiler
    CS.T = transpiler
    if not source:
        raise Unsupported('no source')
    src = CE._strip_comments(source)
    found = CE.class_body(src, cls)
    if not found:
        raise Unsupported('not declared')
    extends, body = found
    if extends != 'TriggeredAbilityImpl':
        raise Unsupported('extends ' + extends)
    bound_fields = set()
    if args:
        body, bound_fields = CE._bind_ctor_args(cls, body, args)
    # Reject stateful instance fields other than static final filters/values or constructor-bound fields
    for m in re.finditer(r'\n\s*(?:private |protected |public )?(?:static )?(?:final )?([\w<>]+) (\w+)\s*(=\s*([^;]*))?;', CE._top_level(body)):
        typ, name, init = m.group(1), m.group(2), m.group(4)
        if typ in ('return',) or '(' in typ or name in bound_fields:
            continue
        if 'static' in m.group(0) and init is not None:
            continue
        raise Unsupported('trigger field ' + name)

    # Parse 0-arg constructor (or constructor whose args were bound): extract super(Zone.BATTLEFIELD, effect[, optional])
    ctor = None
    for m in re.finditer(r'\b' + re.escape(cls) + r'\s*\(([^)]*)\)\s*\{', body):
        pstr = m.group(1).strip()
        if pstr and cls in pstr:
            continue  # copy constructor
        end = CE._matching(body, m.end() - 1, '{', '}')
        ctor = body[m.end():end - 1]
        break
    if ctor is None:
        raise Unsupported('trigger no ctor')

    sup = None
    targets = []
    extra_effects = []
    for node in CE.parse(ctor):
        if node[0] != 'stmt':
            raise Unsupported('trigger ctor control flow')
        st = node[1]
        if st.startswith('super(') and st.endswith(')'):
            sup = CE._split(st[6:-1], ',')
        elif re.match(r'^(?:this\.)?(?:setTriggerPhrase|setRuleVisible|withRuleTextReplacement|addHint|setLeavesTheBattlefieldTrigger)\(', st):
            continue
        elif (tm := re.fullmatch(r'(?:this\.)?addTarget\((.*)\)', st)):
            targets.append(t.target(tm.group(1)))
        elif (em := re.fullmatch(r'(?:this\.)?addEffect\((.*)\)', st)):
            extra_effects.append(t.effect(em.group(1)))
        elif re.fullmatch(r'(?:this\.)?\w+=\w+', st):
            continue  # bound field assignment
        else:
            raise Unsupported('trigger ctor stmt ' + st[:60])

    if not sup or len(sup) not in (2, 3):
        raise Unsupported('trigger super')
    zone = sup[0].strip()
    if zone != 'Zone.BATTLEFIELD':
        raise Unsupported('trigger zone ' + zone)
    effects = [t.effect(sup[1].strip())] + extra_effects
    optional = False
    if len(sup) == 3:
        opt_s = sup[2].strip()
        if opt_s not in ('true', 'false'):
            raise Unsupported('trigger optional ' + opt_s)
        optional = opt_s == 'true'

    # Parse checkEventType
    _, ev_body = CE.method_body(body, 'checkEventType')
    if not ev_body:
        raise Unsupported('trigger no checkEventType')
    events = set(re.findall(r'EventType\.(\w+)', ev_body))
    if len(events) != 1:
        raise Unsupported('trigger events ' + ','.join(sorted(events)))
    event = next(iter(events))

    # Parse checkTrigger strictly
    _, trig_body = CE.method_body(body, 'checkTrigger')
    if not trig_body:
        raise Unsupported('trigger no checkTrigger')
    tc = re.sub(r'\s+', '', trig_body)
    # Disallow target pointer mutation, value setting, watchers, loops, or side effects
    if any(bad in tc for bad in ('setTargetPointer', 'setValue', 'Watcher', 'for(', 'while(', 'addEffect', 'clearTargets', 'adjustTargets')):
        raise Unsupported('trigger side effect')

    fmap = CS.class_filters(t, body)

    def resolve_filter(fname):
        return fmap[fname] if fname in fmap else t.filter(fname)

    on = None
    # 1. Step begins: UPKEEP_STEP_PRE / END_TURN_STEP_PRE / DRAW_STEP_PRE
    step_map = {'UPKEEP_STEP_PRE': 'upkeep', 'END_TURN_STEP_PRE': 'end', 'DRAW_STEP_PRE': 'draw'}
    if event in step_map:
        if tc in ('returntrue;',):
            on = {'e': 'step_begins', 'step': step_map[event], 'who': 'any'}
        elif tc in ('returnevent.getPlayerId().equals(this.controllerId);',
                    'returnevent.getPlayerId().equals(getControllerId());',
                    'returnisControlledBy(event.getPlayerId());',
                    'returnthis.isControlledBy(event.getPlayerId());'):
            on = {'e': 'step_begins', 'step': step_map[event], 'who': 'you'}
        elif tc in ('returngame.getOpponents(this.controllerId).contains(event.getPlayerId());',
                    'returngame.getOpponents(getControllerId()).contains(event.getPlayerId());'):
            on = {'e': 'step_begins', 'step': step_map[event], 'who': 'opponent'}

    # 2. SPELL_CAST
    elif event == 'SPELL_CAST':
        who = None
        if any(p in tc for p in ('isControlledBy(event.getPlayerId())', 'event.getPlayerId().equals(getControllerId())',
                                 'event.getPlayerId().equals(this.controllerId)', 'spell.isControlledBy(getControllerId())')):
            who = 'you'
        elif any(p in tc for p in ('getOpponents(getControllerId()).contains(event.getPlayerId())',
                                   'getOpponents(this.controllerId).contains(event.getPlayerId())')):
            who = 'opponent'
        else:
            who = 'any'
        # Match simple `Spell spell = game.getStack().getSpell(event.getTargetId()); return spell != null && filter.match(spell, ...);`
        fm = re.search(r'([\w.]+)\.match\(spell(?:,[^)]+)?\)', trig_body)
        if fm and len(tc) < 260 and 'if(' not in tc.replace('if(spell==null)returnfalse;', '').replace('if(spell==null){returnfalse;}', ''):
            on = {'e': 'spell_cast', 'filter': resolve_filter(fm.group(1)), 'who': who}

    # 3. ENTERS_THE_BATTLEFIELD
    elif event == 'ENTERS_THE_BATTLEFIELD':
        fm = re.search(r'([\w.]+)\.match\((?:permanent|perm)(?:,[^)]+)?\)', trig_body)
        if fm and len(tc) < 280 and 'getSourceId().equals(event.getTargetId())' not in tc:
            filt = resolve_filter(fm.group(1))
            if any(p in tc for p in ('isControlledBy(getControllerId())', 'getControllerId().equals(permanent.getControllerId())')):
                filt = filt + [{'p': 'controller', 'who': 'you'}]
            on = {'e': 'enters_all', 'filter': filt, 'include_self': '!event.getTargetId().equals(getSourceId())' not in tc and '!permanent.getId().equals(getSourceId())' not in tc}

    # 4. DISCARDED_CARD
    elif event == 'DISCARDED_CARD':
        if tc in ('returnisControlledBy(event.getPlayerId());', 'returnevent.getPlayerId().equals(getControllerId());', 'returnevent.getPlayerId().equals(this.controllerId);'):
            on = {'e': 'discards', 'who': 'you', 'filter': []}
        elif tc in ('returngame.getOpponents(getControllerId()).contains(event.getPlayerId());', 'returngame.getOpponents(this.controllerId).contains(event.getPlayerId());'):
            on = {'e': 'discards', 'who': 'opponent', 'filter': []}
        elif tc == 'returntrue;':
            on = {'e': 'discards', 'who': 'any', 'filter': []}

    # 5. DAMAGED_PLAYER (by this permanent)
    elif event == 'DAMAGED_PLAYER':
        combat_only = 'isCombatDamage()' in tc
        if 'event.getSourceId().equals(getSourceId())' in tc or 'event.getSourceId().equals(this.sourceId)' in tc:
            if len(tc) < 180 and 'getOpponents' not in tc:
                on = {'e': 'deals_combat_damage_player' if combat_only else 'deals_damage_player'}

    if on is None:
        raise Unsupported('trigger body ' + event)

    out = {'class': 'TriggeredAbility', 'java': cls, 'on': on, 'effects': effects}
    if optional:
        out['optional'] = True
    if targets:
        out['targets'] = targets
    return out

//! Windcrag Siege: "As this enchantment enters, choose Mardu or Jeskai. • Mardu — If a creature attacking causes a
//! triggered ability of a permanent you control to trigger, that ability triggers an additional time. • Jeskai — At
//! the beginning of your upkeep, create a 1/1 red Goblin creature token. It gains lifelink and haste until end of
//! turn."

use super::*;
use crate::state::TriggerCause;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "WindcragSiege.AdditionalTriggersAttackingReplacementEffect",
        &[EventType::NumberOfTriggers],
        applies,
        replace,
    )
    .args(&["one_of:false"]),
    Handler::effect("WindcragSiege.WindcragSiegeEffect", goblin).effects(&[
        "new CreateTokenEffect(new GoblinToken())",
        "new GainAbilityTargetEffect(LifelinkAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
    ]),
];

/// `AdditionalTriggersAttackingReplacementEffect(false)` (Isshin's): the trigger's source is a permanent you
/// control, and attackers being declared caused it, whoever's creatures they are
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    matches!(game.trigger_cause, Some(TriggerCause::Attacked))
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield && game.obj(s).controller == cx.controller)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}

/// The Goblin, then lifelink and then haste until end of turn for it (`FixedTargets(token)`: the tokens it made)
fn goblin(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<Target> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(Target::Obj)
        .collect();
    if made.is_empty() {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[1..2], made.clone());
    cx.run_on(game, d, &cx.effects[2..3], made);
    Some(())
}

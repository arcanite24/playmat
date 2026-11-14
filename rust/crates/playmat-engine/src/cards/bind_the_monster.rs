//! Bind the Monster: "When Bind the Monster enters, tap enchanted creature. It deals damage to you equal to its
//! power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BindTheMonster.BindTheMonsterEffect", apply)];

/// the creature the Aura is (or last was) attached to, while it's on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((creature, _)) = game.obj(cx.source_id()).attached_to else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    tap(game, creature);
    let power = game.power_toughness(creature).0;
    deal_damage(game, creature, Target::Player(cx.controller()), power, false);
    Some(())
}

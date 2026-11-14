//! Synchronized Spellcraft: "Synchronized Spellcraft deals 4 damage to target creature and X damage to that
//! creature's controller, where X is the number of creatures in your party."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SynchronizedSpellcraft.SynchronizedSpellcraftEffect", apply)];

/// `PartyCount` first, then `DamageTargetAndTargetControllerEffect(4, X)`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let party: Amount = serde_json::from_value(serde_json::json!({"v": "party"})).ok()?;
    let x = game.value(&party, cx.controller(), cx.source_id(), cx.item.x);
    let Some(o) = cx.obj(0) else { return Some(()) };
    let player = game.obj(o).controller;
    deal_damage(game, cx.source_id(), Target::Obj(o), 4, false);
    if x > 0 {
        deal_damage(game, cx.source_id(), Target::Player(player), x, false);
    }
    Some(())
}

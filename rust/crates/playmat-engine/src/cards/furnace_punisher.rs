//! Furnace Punisher: "Menace. At the beginning of each player's upkeep, Furnace Punisher deals 2 damage to that
//! player unless they control two or more basic lands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FurnacePunisher.FurnacePunisherEffect", apply)];

/// "That player" is the active player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = game.active else { return Some(()) };
    let basics = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.controller == p && game.has_type(**o, CardType::Land) && ob.def.supertypes.iter().any(|s| s == "BASIC")
        })
        .count();
    if basics < 2 {
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}

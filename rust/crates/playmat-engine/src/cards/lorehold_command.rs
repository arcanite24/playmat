//! Lorehold Command: "Choose two — • Create a 3/2 red and white Spirit creature token. • Creatures you control get
//! +1/+0 and gain indestructible and haste until end of turn. • Lorehold Command deals 3 damage to any target. Target
//! player gains 3 life. • Sacrifice a permanent, then draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoreholdCommand.LoreholdCommandEffect", apply)];

/// 3 damage to the mode's first target (a player or a permanent), then 3 life for its second, the target player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    match cx.target(0) {
        Some(Target::Player(p)) => {
            deal_damage(game, cx.source_id(), Target::Player(p), 3, false);
        }
        Some(Target::Obj(o)) if game.obj(o).zone == Zone::Battlefield => {
            deal_damage(game, cx.source_id(), Target::Obj(o), 3, false);
        }
        _ => {}
    }
    if let Some(p) = cx.player(1) {
        gain_life(game, p, 3);
    }
    Some(())
}

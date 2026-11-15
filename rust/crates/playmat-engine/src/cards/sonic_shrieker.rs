//! Sonic Shrieker: "When this creature enters, it deals 2 damage to any target and you gain 2 life. If a player is
//! dealt damage this way, they discard a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SonicShrieker.SonicShriekerEffect", apply)];

/// 2 damage to the target, 2 life, and a discard by a player the damage reached
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let hit = match cx.target(0) {
        Some(Target::Obj(o)) => {
            deal_damage(game, cx.source_id(), Target::Obj(o), 2, false);
            None
        }
        Some(Target::Player(p)) => (deal_damage(game, cx.source_id(), Target::Player(p), 2, false) > 0).then_some(p),
        None => None,
    };
    gain_life(game, cx.controller(), 2);
    if let Some(p) = hit {
        discard(game, d, p, 1);
    }
    Some(())
}

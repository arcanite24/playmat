//! Fevered Visions: "At the beginning of each player's end step, that player draws a card. If the player is your
//! opponent and has four or more cards in hand, Fevered Visions deals 2 damage to that player."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FeveredVisions.FeveredVisionsEffect", apply)];

/// The active player draws; an opponent of yours with more than three cards in hand then takes 2 from this
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = game.active else { return Some(()) };
    draw(game, p, 1);
    if p != cx.controller() && game.player(p).hand.len() > 3 {
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}

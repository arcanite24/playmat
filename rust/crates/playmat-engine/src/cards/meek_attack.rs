//! Meek Attack: "{1}{R}: You may put a creature card with total power and toughness 5 or less from your hand onto
//! the battlefield. That creature gains haste. At the beginning of the next end step, sacrifice that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MeekAttack.MeekAttackPredicate", small)];

/// the card's power plus its toughness, 5 or less (a card in hand: its own)
fn small(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let (p, t) = game.power_toughness(id);
    p + t <= 5
}

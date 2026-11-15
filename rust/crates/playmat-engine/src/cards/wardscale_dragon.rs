//! Wardscale Dragon: "As long as Wardscale Dragon is attacking, defending player can't cast spells."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("WardscaleDragon.WardscaleDragonRuleEffect", Rule::CantCast(defending_player))];

/// the source attacking, and the caster the player it attacks (or whose planeswalker it attacks)
fn defending_player(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, _: &CardDef) -> bool {
    let o = game.obj(cx.source);
    o.zone == Zone::Battlefield && o.attacking == Some(seat)
}

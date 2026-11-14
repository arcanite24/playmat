//! Phyrexian Censor: "Each player can't cast more than one non-Phyrexian spell each turn." (`CAST_SPELL`, with
//! `PhyrexianCensorWatcher` counting each player's non-Phyrexian spells this turn)

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[Handler::rule("PhyrexianCensor.PhyrexianCensorEffect", Rule::CantCast(cant_cast))];

fn cant_cast(game: &Game, _cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    // `game.getCard(event.getSourceId())`: the card, whatever face is cast
    !game.has_subtype(id, "Phyrexian")
        && game.watch.spells.iter().any(|s| s.caster == seat && !game.has_subtype(s.card, "Phyrexian"))
}

//! Raffine's Guidance: "Enchant creature. Enchanted creature gets +1/+1. You may cast Raffine's Guidance from your
//! graveyard by paying {2}{W} rather than paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("RaffinesGuidance.RafinnesGuidancePlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {2}{W}
/// (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard)
        .then(|| PlayWay { mana: ManaCost::parse("{2}{W}").ok(), costs: Vec::new() })
}

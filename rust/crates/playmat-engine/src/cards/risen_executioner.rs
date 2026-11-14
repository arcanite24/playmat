//! Risen Executioner: "Risen Executioner can't block. Other Zombie creatures you control get +1/+1. You may cast
//! Risen Executioner from your graveyard if you pay {1} more to cast it for each other creature card in your
//! graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("RisenExecutioner.RisenExecutionerCastEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, by its owner, from their graveyard, for its mana cost plus {1} for each
/// other creature card there (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    if id != cx.source || o.owner != seat || o.zone != Zone::Graveyard {
        return None;
    }
    let others = game
        .player(cx.controller)
        .graveyard
        .iter()
        .filter(|c| **c != id && game.obj(**c).def.is(CardType::Creature))
        .count() as u32;
    let mut mana = face.cost.clone();
    mana.generic += others;
    Some(PlayWay { mana: Some(mana), costs: Vec::new() })
}

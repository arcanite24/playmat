//! Nightveil Specter: "Whenever Nightveil Specter deals combat damage to a player, that player exiles the top card of
//! their library. You may play cards exiled with Nightveil Specter."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("NightveilSpecter.NightveilSpecterExileEffect", exile),
    Handler::as_though("NightveilSpecter.NightveilSpecterEffect", AsThough::Play(play)),
];

/// The damaged player's top card, exiled into this one's zone (`getCardExileZoneId`: as it is now)
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let &card = game.player(p).library.first()?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let src = cx.source_id();
        let zcc = game.obj(src).zcc;
        game.exiled_for_source.push((src, zcc, card));
    }
    Some(())
}

/// `PLAY_FROM_NOT_OWN_HAND_ZONE`: a card in this one's exile zone (as it is now), by its controller
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let zcc = game.obj(cx.source).zcc;
    (seat == cx.controller
        && game.obj(id).zone == Zone::Exile
        && game.exiled_for_source.iter().any(|(s, z, o)| *s == cx.source && *z == zcc && *o == id))
        .then(PlayWay::default)
}

//! Screaming Swarm: "Flying. Whenever you attack with one or more creatures, target player mills that many cards.
//! {2}{U}: Put Screaming Swarm from your graveyard into your library second from the top."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("ScreamingSwarm.ScreamingSwarmValue", attackers),
    Handler::effect("ScreamingSwarm.ScreamingSwarmEffect", apply),
];

/// `VALUEKEY_NUMBER_ATTACKERS`: the creatures you attacked with (the engine's turns have one combat, so this
/// turn's declared attackers)
fn attackers(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.player(you).attackers_this_turn as i32
}

/// The card, still the one in the graveyard that activated it, goes into its owner's library second from the top
/// (`putCardOnTopXOfLibrary(card, 2, withName)`: on the bottom of an empty library)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    if game.obj(id).zone != Zone::Graveyard || game.obj(id).zcc != cx.source.1 {
        return Some(());
    }
    let owner = game.obj(id).owner;
    if game.player(owner).library.is_empty() {
        to_library(game, id, false);
        return Some(());
    }
    game.move_to(id, Zone::Library);
    if game.obj(id).zone != Zone::Library {
        return Some(());
    }
    let lib = &mut game.player_mut(owner).library;
    lib.retain(|x| *x != id);
    lib.insert(1, id);
    if log::kept(game) {
        let line = format!("{} is put into {}'s library second from the top", log::object(game, id), log::player(owner));
        log::inform(game, line);
    }
    Some(())
}

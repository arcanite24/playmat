//! Etrata, the Silencer: "Etrata can't be blocked. Whenever Etrata deals combat damage to a player, exile target
//! creature that player controls and put a hit counter on that card. That player loses the game if they own three
//! or more exiled cards with hit counters on them. Etrata's owner shuffles Etrata into their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EtrataTheSilencer.EtrataTheSilencerEffect", apply)];

/// The target exiled and a hit counter on the card; its controller loses (unless they can't) with three or more
/// exiled cards they own with hit counters; then Etrata, still on the battlefield, is shuffled into its owner's
/// library, or else your library is shuffled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(creature) = cx.obj(0) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let player = game.obj(creature).controller;
    game.move_to(creature, Zone::Exile);
    if game.obj(creature).zone == Zone::Exile && game.exile.contains(&creature) {
        game.obj_mut(creature).add_counters("hit", 1);
    }
    let found = game.exile.iter().filter(|c| game.obj(**c).owner == player && game.obj(**c).counter("hit") > 0).count();
    if found >= 3 && !game.player(player).lost && !game.over && !cards::cant_lose_or_win(game, player, false) {
        game.player_mut(player).lost = true;
        if log::kept(game) {
            let line = format!("{} has lost the game", log::player(player));
            log::inform(game, line);
        }
    }
    if cx.source_on_battlefield(game) {
        cx.run(game, d, &[EffectDef::ShuffleIntoLibrarySourceEffect]);
        return Some(());
    }
    let you = cx.controller();
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

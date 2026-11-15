//! Hoarding Broodlord: "Convoke. Flying. When Hoarding Broodlord enters, search your library for a card, exile it
//! face down, then shuffle. For as long as that card remains exiled, you may play it. Spells you cast from exile have
//! convoke."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HoardingBroodlord.HoardingBroodlordEffect", apply)];

/// `searchLibrary(TargetCardInLibrary())`, the shuffle, then the card found exiled face down, playable by you for as
/// long as it stays exiled (`makeCardPlayable(…, Duration.Custom, …)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &crate::card::Filter::default(), 1, 1, cx.source_id())?;
    let card = found.first().copied().filter(|c| game.obj(*c).zone == Zone::Library);
    shuffle_library(game, you);
    let Some(c) = card else { return Some(()) };
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone == Zone::Exile {
        let o = game.obj_mut(c);
        o.may_play = Some((you, u32::MAX));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.exiled_face_down = true;
    }
    Some(())
}

fn shuffle_library(game: &mut Game, seat: Seat) {
    let mut library = game.player(seat).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(seat).library = library;
}

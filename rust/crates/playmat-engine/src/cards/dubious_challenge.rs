//! Dubious Challenge: "Look at the top ten cards of your library, exile up to two creature cards from among them,
//! then shuffle. Target opponent may choose one of the exiled cards and put it onto the battlefield under their
//! control. Put the rest onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DubiousChallenge.DubiousChallengeEffect", apply).filters(&["StaticFilters.FILTER_CARD_CREATURE"]),
    // its two `DubiousChallengeMoveToBattlefieldEffect`s only move what the first effect set on them: done there
    Handler::effect("DubiousChallenge.DubiousChallengeMoveToBattlefieldEffect", moved_already),
];

/// up to two creature cards of the top ten (`player.choose`), exiled, then the shuffle (also with none chosen); the
/// target opponent chooses up to one of them (`TargetCard(0, 1, EXILED)`), which goes onto the battlefield under
/// their control, then the rest under yours (the spell's second and third effects, which follow at once)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let top: Vec<ObjId> = game.player(you).library.iter().take(10).copied().collect();
    let creatures: Vec<ObjId> =
        top.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), *c)).collect();
    let chosen =
        if top.is_empty() { Vec::new() } else { super::gilgamesh_master_at_arms::choose_any(game, d, you, &creatures, 2)? };
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Exiled) {
        return None;
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    if chosen.is_empty() {
        return Some(());
    }
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let mut exiled = chosen;
    let possible: Vec<ObjId> = exiled
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Exile && game.filter_matches(&cx.filters[0], opponent, Some(src), *c))
        .collect();
    let theirs = super::gilgamesh_master_at_arms::choose_any(game, d, opponent, &possible, 1)?;
    exiled.retain(|c| !theirs.contains(c));
    if !theirs.is_empty() && !put_cards(game, d, opponent, &theirs, crate::card::PutCards::Battlefield) {
        return None;
    }
    if !exiled.is_empty() && !put_cards(game, d, you, &exiled, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

fn moved_already(_: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    Some(())
}

//! `SearchTargetGraveyardHandLibraryForCardNameAndExileEffect` (Slaughter Games, Unmoored Ego, Lost Legacy,
//! Dispossess, Ancient Vendetta, Stain the Mind): "search target player's graveyard, hand, and library for any number
//! of cards with that name and exile them. That player shuffles (then draws a card for each card exiled from their
//! hand this way)". The name is the one `ChooseACardNameEffect` kept on the spell.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SlaughterGames.SlaughterGamesEffect", slaughter_games),
    Handler::effect("UnmooredEgo.UnmooredEgoEffect", unmoored_ego),
    Handler::effect("LostLegacy.LostLegacyEffect", lost_legacy),
    Handler::effect("Dispossess.DispossessEffect", slaughter_games),
    Handler::effect("AncientVendetta.AncientVendettaEffect", ancient_vendetta),
    Handler::effect("StainTheMind.StainTheMindEffect", slaughter_games),
];

fn slaughter_games(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    search_and_exile(game, d, cx, usize::MAX, false)
}

fn unmoored_ego(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    search_and_exile(game, d, cx, 4, true)
}

fn lost_legacy(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    search_and_exile(game, d, cx, usize::MAX, true)
}

fn ancient_vendetta(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    search_and_exile(game, d, cx, 4, false)
}

/// `HumanPlayer.choose(outcome, cards, target)` with a `TargetCard(min, max)` of the cards named so: nothing asked
/// when none has the name (max 0)
fn choose_named(
    game: &mut Game,
    d: &mut dyn Decider,
    you: Seat,
    zone: &[ObjId],
    named: &[ObjId],
    min: usize,
    max: usize,
) -> Option<Vec<ObjId>> {
    if zone.is_empty() || max == 0 {
        return Some(Vec::new());
    }
    choose_cards(game, d, you, zone, named, min, max, "exile")
}

/// `applySearchAndExile`, its graveyard choice optional
fn search_and_exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, most: usize, draw_for_hand: bool) -> Option<()> {
    let you = cx.controller();
    let Some(name) = game.obj(cx.source_id()).chosen_name.clone() else { return Some(()) };
    let Some(p) = cx.player(0) else { return Some(()) };
    let named = |game: &Game, zone: &[ObjId]| -> Vec<ObjId> {
        zone.iter().copied().filter(|c| game.obj(*c).def.name == name).collect()
    };
    let mut left = most;
    // the graveyard: asked only when a card of the name is there
    let grave = game.player(p).graveyard.clone();
    let in_grave = named(game, &grave);
    let n = in_grave.len().min(left);
    if n > 0 {
        let chosen = choose_named(game, d, you, &grave, &in_grave, 0, n)?;
        left -= chosen.len();
        for c in chosen {
            game.move_to(c, Zone::Exile);
        }
    }
    // the hand
    let hand = game.player(p).hand.clone();
    let in_hand = named(game, &hand);
    let n = in_hand.len().min(left);
    let chosen = choose_named(game, d, you, &hand, &in_hand, 0, n)?;
    left -= chosen.len();
    let from_hand = if draw_for_hand { chosen.len() } else { 0 };
    for c in chosen {
        game.move_to(c, Zone::Exile);
    }
    // the library
    let library = game.player(p).library.clone();
    let in_library = named(game, &library);
    let n = in_library.len().min(left);
    let chosen = choose_named(game, d, you, &library, &in_library, 0, n)?;
    for c in chosen {
        game.move_to(c, Zone::Exile);
    }
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    if from_hand > 0 {
        draw(game, p, from_hand);
    }
    Some(())
}

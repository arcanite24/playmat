//! Phantom Carriage: "Flying. When Phantom Carriage enters, you may search your library for a card with flashback or
//! disturb, put it into your graveyard, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PhantomCarriage.PhantomCarriageEffect", apply)];

/// The search (`TargetCardInLibrary(filter)`: one card with flashback or disturb), the card found to the graveyard,
/// then the shuffle (searched or not)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([{"p": "or", "xs": [
        {"p": "ability_class", "class": "FlashbackAbility"},
        {"p": "ability_class", "class": "DisturbAbility"}]}]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.move_to(card, Zone::Graveyard);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

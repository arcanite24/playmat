//! Say Its Name: "Mill three cards. Then you may return a creature or land card from your graveyard to your hand.
//! Exile this card and two other cards named Say Its Name from your graveyard: Search your graveyard, hand, and/or
//! library for a card named Altanak, the Thrice-Called and put it onto the battlefield. If you search your library
//! this way, shuffle. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SayItsName.SayItsNameSearchEffect", apply)];

const ALTANAK: &str = "Altanak, the Thrice-Called";

/// "Search your graveyard?" (yes: choose one Altanak there, `TargetCardInYourGraveyard(1, 1, …)`), then with none
/// found "Search your hand?" (up to one there), then "Search your library?" (up to one, then the shuffle); the card
/// found is put onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let named = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| game.obj(*c).name() == ALTANAK).collect()
    };
    let mut card = None;
    if choose_use(game, d, you, "Search your graveyard?")? {
        let gy = game.player(you).graveyard.clone();
        let possible = named(game, &gy);
        card = choose_cards(game, d, you, &gy, &possible, 1, 1, "card named Altanak, the Thrice-Called")?.first().copied();
    }
    if card.is_none() && choose_use(game, d, you, "Search your hand?")? {
        let hand = game.player(you).hand.clone();
        let possible = named(game, &hand);
        card = choose_cards(game, d, you, &hand, &possible, 0, 1, "card named Altanak, the Thrice-Called")?.first().copied();
    }
    if card.is_none() && choose_use(game, d, you, "Search your library?")? {
        let filter: crate::card::Filter =
            serde_json::from_value(serde_json::json!([{"p": "name", "n": ALTANAK}])).ok()?;
        card = search_library(game, d, you, &filter, 0, 1, cx.source_id())?.first().copied();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    if let Some(c) = card {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}

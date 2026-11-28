//! Specter's Shriek: "Target opponent reveals their hand. You may choose a nonland card from it. If you do, that
//! player exiles that card. If a nonblack card is exiled this way, exile a card from your hand."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpectersShriek.SpectersShriekEffect", apply)];

/// The hand revealed; with a nonland card there, "Exile a card from …'s hand?", then up to one nonland card
/// (`TargetCard(0, 1, …)` with `Player.choose`) exiled; a nonblack one, and you exile a card from your hand (one,
/// unless your hand is empty)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if nonland.is_empty() {
        return Some(());
    }
    let message = format!("Exile a card from {}'s hand?", log::player(p));
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &hand, &nonland, 0, 1, &rule)?.first() else { return Some(()) };
    let black = game.colors(card).contains(&Color::B);
    game.move_to(card, Zone::Exile);
    let mine = game.player(you).hand.clone();
    if black || mine.is_empty() {
        return Some(());
    }
    if let Some(&c) = choose_cards(game, d, you, &mine, &mine, 1, 1, &rule)?.first() {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}

//! Inscribed Tablet: "{1}, {T}, Sacrifice this artifact: Reveal the top five cards of your library. Put a land card
//! from among them into your hand and the rest on the bottom of your library in a random order. If you didn't put a
//! card into your hand this way, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InscribedTablet.InscribedTabletEffect", apply)];

/// `chooseTarget(cards, TargetCard(land))` when there's a land among them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let lands: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).is(CardType::Land)).collect();
    let mut to_hand = false;
    let mut rest = cards.clone();
    if !lands.is_empty() {
        let rule = item_rule(game, cx.item);
        if let Some(l) = choose_cards(game, d, you, &cards, &lands, 1, 1, &rule)?.first().copied() {
            rest.retain(|c| *c != l);
            game.move_to(l, Zone::Hand);
            to_hand = game.obj(l).zone == Zone::Hand;
        }
    }
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    if !to_hand {
        draw(game, you, 1);
    }
    Some(())
}

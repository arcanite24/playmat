//! Karn, Scion of Urza: "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that
//! card into your hand and exile the other with a silver counter on it. −1: Put a card you own with a silver counter
//! on it from exile into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KarnScionOfUrza.KarnPlus1Effect", plus_one),
    Handler::effect("KarnScionOfUrza.KarnMinus1Effect", minus_one),
];

/// One card revealed goes to your hand; two, the opponent picks (`chooseTarget(TargetCard(1, LIBRARY))`) and the
/// other is exiled with a silver counter
fn plus_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let to_hand = if cards.len() == 1 {
        Some(cards[0])
    } else {
        let opponent = game.opponent(you);
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, opponent, &cards, &cards, 1, 1, &rule)?.first().copied()
    };
    if let Some(c) = to_hand {
        game.move_to(c, Zone::Hand);
    }
    for c in cards {
        if game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Exile);
            if game.obj(c).zone == Zone::Exile {
                game.obj_mut(c).add_counters("silver", 1);
            }
        }
    }
    Some(())
}

/// The cards in exile you own with a silver counter: one is taken as it is, more you choose among
/// (`TargetCardInExile`, not targeted)
fn minus_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> =
        game.exile.iter().copied().filter(|c| game.obj(*c).owner == you && game.obj(*c).counter("silver") > 0).collect();
    let card = match cards.len() {
        0 => return Some(()),
        1 => cards[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    game.move_to(card, Zone::Hand);
    Some(())
}

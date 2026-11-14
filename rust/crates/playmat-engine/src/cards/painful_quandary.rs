//! Painful Quandary: "Whenever an opponent casts a spell, that player loses 5 life unless they discard a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PainfulQuandary.PainfulQuandryEffect", apply)];

/// With a card in hand, the player is asked; yes pays the discard cost (a card they choose), else 5 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let mut paid = false;
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() && choose_use(game, d, p, "Discard a card (otherwise you lose 5 life)?")? {
        let chosen = choose_cards(game, d, p, &hand, &hand, 1, 1, "discard")?;
        for c in &chosen {
            discard_card(game, p, *c);
        }
        game.discarded_cards();
        paid = !chosen.is_empty();
    }
    if !paid {
        lose_life(game, p, 5);
    }
    Some(())
}

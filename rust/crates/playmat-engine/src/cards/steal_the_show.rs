//! Steal the Show: "Choose one or both — • Target player discards any number of cards, then draws that many cards.
//! • Steal the Show deals damage equal to the number of instant and sorcery cards in your graveyard to target creature
//! or planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StealTheShow.StealTheShowEffect", apply)];

/// `player.discard(0, MAX)`: `TargetDiscard(0, MAX).choose` rounds over the hand (nothing asked of an empty one),
/// then they draw that many
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    let chosen = if hand.is_empty() {
        Vec::new()
    } else {
        choose_cards_rounds(game, d, p, &hand, 0, hand.len())?
    };
    for c in &chosen {
        discard_card(game, p, *c);
    }
    game.discarded_cards();
    draw(game, p, chosen.len());
    Some(())
}

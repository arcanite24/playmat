//! Manhole Missile: "Manhole Missile deals 3 damage to target creature. You may put a card from your hand on the
//! bottom of your library. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ManholeMissile.ManholeMissileEffect", apply)];

/// "Put a card from your hand on the bottom of your library?" (not with an empty hand); a card of the hand
/// (`TargetCardInHand`, `Player.choose`) to the bottom, then a draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return None;
    }
    let q = Query::YesNo { message: "Put a card from your hand on the bottom of your library?".into() };
    let yes = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if !yes {
        return None;
    }
    let rule = item_rule(game, cx.item);
    let &c = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first()?;
    if !put_cards(game, d, you, &[c], crate::card::PutCards::BottomAny) {
        return None;
    }
    draw(game, you, 1);
    Some(())
}

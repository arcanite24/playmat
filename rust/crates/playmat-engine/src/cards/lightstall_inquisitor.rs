//! Lightstall Inquisitor: "When this creature enters, each opponent exiles a card from their hand and may play that
//! card for as long as it remains exiled. Each spell cast this way costs {1} more to cast. Each land played this way
//! enters tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LightstallInquisitor.LightstallInquisitorEffect", apply)];

/// Each opponent in turn order with a card in hand chooses one (`TargetCardInHand`) and exiles it; they may play it
/// while it stays exiled, {1} more as a spell, tapped as a land
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let rule = item_rule(game, cx.item);
    for p in opponents {
        let hand = game.player(p).hand.clone();
        if hand.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, p, &hand, &hand, 1, 1, &rule)?;
        for c in chosen {
            game.move_to(c, Zone::Exile);
            if game.obj(c).zone != Zone::Exile {
                continue;
            }
            let o = game.obj_mut(c);
            o.may_play = Some((p, u32::MAX));
            o.pay_tax = 1;
            o.play_tapped = true;
        }
    }
    Some(())
}

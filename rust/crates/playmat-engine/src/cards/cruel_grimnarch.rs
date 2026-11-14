//! Cruel Grimnarch: "When Cruel Grimnarch enters the battlefield, each opponent discards a card. For each opponent
//! who can't, you gain 4 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CruelGrimnarch.CruelGrimnarchEffect", apply)];

/// Each opponent picks a card to discard (`TargetDiscard`, `chooseTarget`); then each discards it, and you gain 4
/// for each who discarded none
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut picks = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let hand = game.player(p).hand.clone();
        let n = hand.len().min(1);
        let chosen = choose_cards(game, d, p, &hand, &hand, n, n, "discard")?;
        picks.push((p, chosen));
    }
    for (p, chosen) in picks {
        for c in &chosen {
            discard_card(game, p, *c);
        }
        game.discarded_cards();
        if chosen.is_empty() {
            gain_life(game, you, 4);
        }
    }
    Some(())
}

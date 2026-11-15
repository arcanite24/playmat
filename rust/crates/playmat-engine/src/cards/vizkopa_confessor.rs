//! Vizkopa Confessor: "Extort. When Vizkopa Confessor enters, pay any amount of life. Target opponent reveals that
//! many cards from their hand. You choose one of them and exile it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VizkopaConfessor.VizkopaConfessorEffect", apply)];

/// `getAmount(0, life, "Pay how many life?")` (not asked with no life); with some paid, the opponent chooses that
/// many cards of their hand (`TargetCard(count, HAND)`, `chooseTarget`), reveals them, and you choose one of them
/// to exile
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let max = game.player(you).life;
    let pay = if max <= 0 {
        0
    } else {
        loop {
            match ask(game, d, you, &Query::Amount { min: 0, max }) {
                Answer::Int(n) if (0..=max).contains(&n) => break n,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
    };
    if pay <= 0 {
        return Some(());
    }
    lose_life(game, you, pay);
    if log::kept(game) {
        let line = format!("{}: {} pays {} life", game.obj(cx.source_id()).name(), log::player(you), pay);
        log::inform(game, line);
    }
    let hand = game.player(opponent).hand.clone();
    let count = hand.len().min(pay as usize);
    let rule = item_rule(game, cx.item);
    let revealed = choose_cards(game, d, opponent, &hand, &hand, count, count, &rule)?;
    if revealed.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = revealed.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    if let Some(c) = choose_cards(game, d, you, &revealed, &revealed, 1, 1, &rule)?.first().copied() {
        if game.obj(c).zone == Zone::Hand {
            game.move_to(c, Zone::Exile);
        }
    }
    Some(())
}

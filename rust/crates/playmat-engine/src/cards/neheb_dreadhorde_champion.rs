//! Neheb, Dreadhorde Champion: "Whenever Neheb deals combat damage to a player or planeswalker, you may discard any
//! number of cards. If you do, draw that many cards and add that much {R}. Until end of turn, you don't lose this mana
//! as steps and phases end."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NehebDreadhordeChampion.NehebDreadhordeChampionEffect", apply)];

/// `discard(0, MAX)`: `TargetDiscard` chosen with `TargetImpl.choose`, discarded in that order; then as many drawn
/// and as much {R}, kept until the turn ends
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, you, &hand, 0, usize::MAX)?;
    for c in &chosen {
        discard_card(game, you, *c);
    }
    if chosen.is_empty() {
        return Some(());
    }
    game.discarded_cards();
    let n = chosen.len();
    draw(game, you, n);
    let p = game.player_mut(you);
    p.mana_pool.colored[3] += n as u32;
    p.eot_mana.colored[3] += n as u32;
    Some(())
}

//! Settle the Score: "Exile target creature. Put two loyalty counters on a planeswalker you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SettleTheScore.SettleTheScoreEffect", apply)];

/// `TargetPermanent.choose` of a planeswalker you control (not targeted), two loyalty counters on it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let walkers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Planeswalker))
        .collect();
    if walkers.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_in_rounds(game, d, you, &walkers, 1, 1, &rule)?;
    if let Some(&w) = chosen.first() {
        if game.obj(w).zone == Zone::Battlefield {
            add_counters_by(game, w, "loyalty", 2, Some(you));
        }
    }
    Some(())
}

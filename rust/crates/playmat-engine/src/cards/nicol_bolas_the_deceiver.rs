//! Nicol Bolas, the Deceiver: "+3: Each opponent loses 3 life unless that player sacrifices a nonland permanent or
//! discards a card. −3: Destroy target creature. Draw a card. −11: Nicol Bolas deals 7 damage to each opponent. You
//! draw seven cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("NicolBolasTheDeceiver.NicolBolasTheDeceiverFirstEffect", apply)];

/// For each opponent: with a nonland permanent, "Sacrifices a nonland permanent?" and one chosen (`Player.choose`)
/// and sacrificed, which ends the effect; else with a card in hand "Discard a card?" and one discarded, which ends
/// it; else they lose 3 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    for p in (1..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let nonland: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land))
            .collect();
        if !nonland.is_empty() && choose_use(game, d, p, "Sacrifices a nonland permanent?")? {
            let rule = item_rule(game, cx.item);
            if let Some(o) = choose_cards(game, d, p, &nonland, &nonland, 1, 1, &rule)?.first().copied() {
                if game.obj(o).zone == Zone::Battlefield {
                    sacrifice(game, o);
                    return Some(());
                }
            }
        }
        if !game.player(p).hand.is_empty() && choose_use(game, d, p, "Discard a card?")? {
            discard(game, d, p, 1);
            if d.stopped() {
                return None;
            }
            return Some(());
        }
        lose_life(game, p, 3);
    }
    Some(())
}

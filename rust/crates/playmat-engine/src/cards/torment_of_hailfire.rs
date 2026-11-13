//! Torment of Hailfire: "Repeat the following process X times. Each opponent loses 3 life unless that player
//! sacrifices a nonland permanent or discards a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TormentOfHailfire.TormentOfHailfireEffect", apply)];

/// X times, for each opponent: with a nonland permanent, "sacrifice one?" (then they choose it); else, with a card
/// in hand, "discard one?"; else they lose 3 life (Rottenmouth Viper's process)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let repeat = cx.item.x;
    for i in 1..=repeat {
        let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
        for p in opponents {
            let nonland: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|b| game.obj(*b).controller == p && !game.has_type(*b, CardType::Land))
                .collect();
            if !nonland.is_empty()
                && choose_use(game, d, p, &format!("Sacrifices a nonland permanent? (Iteration {i} of {repeat})"))?
            {
                let chosen = choose_cards(game, d, p, &nonland, &nonland, 1, 1, &item_rule(game, cx.item))?;
                if let Some(c) = chosen.first().copied() {
                    sacrifice(game, c);
                    continue;
                }
            }
            if !game.player(p).hand.is_empty()
                && choose_use(game, d, p, &format!("Discard a card? (Iteration {i} of {repeat})"))?
            {
                discard(game, d, p, 1);
                continue;
            }
            lose_life(game, p, 3);
        }
    }
    Some(())
}

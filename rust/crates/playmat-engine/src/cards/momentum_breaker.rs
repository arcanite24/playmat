//! Momentum Breaker: "When this enchantment enters, each opponent sacrifices a creature or Vehicle of their choice.
//! Each opponent who can't discards a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MomentumBreaker.MomentumBreakerEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let opp = (you + i) % game.seats();
        if game.player(opp).lost {
            continue;
        }
        // `SacrificeEffect` with a `TargetSacrifice` of one; false (nothing sacrificed) and they discard
        let possible: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| {
                game.obj(*o).controller == opp
                    && (game.has_type(*o, CardType::Creature) || game.has_subtype(*o, "Vehicle"))
            })
            .collect();
        let mut sacrificed = false;
        if !possible.is_empty() {
            let chosen = choose_cards(game, d, opp, &possible, &possible, 1, 1, "sacrifice");
            let Some(chosen) = chosen else { return None };
            for o in chosen {
                if game.obj(o).zone == Zone::Battlefield {
                    sacrifice(game, o);
                    sacrificed |= game.obj(o).zone != Zone::Battlefield;
                }
            }
        }
        if !sacrificed {
            discard(game, d, opp, 1);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}

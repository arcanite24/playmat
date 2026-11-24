//! Zoyowa Lava-Tongue: "Deathtouch. At the beginning of your end step, if you descended this turn, each opponent
//! may discard a card or sacrifice a permanent. Zoyowa Lava-Tongue deals 3 damage to each opponent who didn't."

use super::*;
use crate::card::{CostDef, Filter};

pub const HANDLERS: &[Handler] = &[Handler::effect("ZoyowaLavaTongue.ZoyowaLavaTongueEffect", apply)];

/// each opponent in turn order from you: the `OrCost` (discard a card or sacrifice a permanent) if it can be paid
/// and they choose to; those who didn't pay are each dealt 3 damage afterwards
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let cost = CostDef::OrCost {
        costs: vec![
            CostDef::DiscardCardCost { filter: Filter::default(), amount: 1, different_names: false },
            CostDef::SacrificeTargetCost { filter: Filter::default(), amount: 1 },
        ],
    };
    let mut unpaid = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let chose = can_pay_cost(game, p, src, &cost)
            && choose_use(game, d, p, "Do you wish to discard a card or sacrifice a permanent to avoid 3 damage to you?")?;
        if chose && pay_cost(game, d, p, src, &cost) {
            continue;
        }
        if d.stopped() {
            return None;
        }
        unpaid.push(p);
    }
    for p in unpaid {
        deal_damage(game, src, Target::Player(p), 3, false);
    }
    Some(())
}

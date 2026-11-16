//! Osseous Sticktwister: "Lifelink. Delirium — At the beginning of your end step, if there are four or more card types
//! among cards in your graveyard, each opponent may sacrifice a nonland permanent of their choice or discard a card.
//! Then Osseous Sticktwister deals damage equal to its power to each opponent who didn't sacrifice a permanent or
//! discard a card this way."

use super::*;
use crate::card::{CostDef, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("OsseousSticktwister.OsseousSticktwisterEffect", apply)];

/// The power of the source (or as it last existed) fixed first; each opponent in turn order from you: the `OrCost`
/// (sacrifice a nonland permanent or discard a card) if it can be paid and they choose to; afterwards those who didn't
/// pay are each dealt that much damage (none when it's 0 or less)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let o = game.obj(src);
    let dmg = match o.power_lki {
        Some(p) if o.zone != Zone::Battlefield => p,
        _ => game.power_toughness(src).0,
    };
    let cost = CostDef::OrCost {
        costs: vec![
            CostDef::SacrificeTargetCost {
                filter: Filter(vec![Pred::Not { x: Box::new(Pred::Type { t: CardType::Land }) }]),
                amount: 1,
            },
            CostDef::DiscardCardCost { filter: Filter::default(), amount: 1 },
        ],
    };
    let message = format!("Sacrifice a nonland permanent or discard a card to avoid {dmg} damage?");
    let mut unpaid = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let chose = can_pay_cost(game, p, src, &cost) && choose_use(game, d, p, &message)?;
        if chose && pay_cost(game, d, p, src, &cost) {
            continue;
        }
        if d.stopped() {
            return None;
        }
        unpaid.push(p);
    }
    if dmg <= 0 {
        return Some(());
    }
    for p in unpaid {
        deal_damage(game, src, Target::Player(p), dmg, false);
    }
    Some(())
}

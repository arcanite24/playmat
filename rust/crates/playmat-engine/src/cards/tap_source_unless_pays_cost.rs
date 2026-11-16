//! `TapSourceUnlessPaysEffect(cost)` resolving, with a mana or sacrifice cost (Heavyweight Demolisher's "at the
//! beginning of your upkeep, tap this unless you pay {3}", Apocalypse Demon's "… unless you sacrifice another
//! creature")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TapSourceUnlessPaysEffect", apply)
    .args(&["one_of:new GenericManaCost(3)|new SacrificeTargetCost(StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE)"])
    .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE"])];

/// `TapSourceUnlessPaysEffect.apply`: with the source on the battlefield, "<cost>? (otherwise <name> becomes
/// tapped)" when the cost can be paid, the payment, and the tap unless it was paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let name = game.obj(src).name().to_string();
    let paid = match cx.args.first().map(|a| a.as_str()) {
        // (a mana cost's `canPay` is always true)
        Some("new GenericManaCost(3)") => {
            choose_use(game, d, you, &format!("{{3}}? (otherwise {name} becomes tapped)"))?
                && pay_mana(game, d, you, &ManaCost::parse("{3}").ok()?)
        }
        Some(_) => {
            let cost = CostDef::SacrificeTargetCost { filter: cx.filters.first()?.clone(), amount: 1 };
            can_pay_cost(game, you, src, &cost)
                && choose_use(game, d, you, &format!("sacrifice another creature? (otherwise {name} becomes tapped)"))?
                && pay_cost(game, d, you, src, &cost)
        }
        None => return Some(()),
    };
    if paid {
        return Some(());
    }
    if d.stopped() {
        return None;
    }
    if game.obj(src).zone == Zone::Battlefield {
        tap(game, src);
    }
    Some(())
}

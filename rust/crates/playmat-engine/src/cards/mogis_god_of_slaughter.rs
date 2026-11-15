//! Mogis, God of Slaughter: "Indestructible. As long as your devotion to black and red is less than seven, Mogis isn't
//! a creature. At the beginning of each opponent's upkeep, Mogis deals 2 damage to that player unless they sacrifice
//! a creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MogisGodOfSlaughter.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply)
    .effects(&["new DamageTargetEffect(2)"])
    .filters(&["StaticFilters.FILTER_PERMANENT_A_CREATURE"])];

/// `DoUnlessTargetPlayerOrTargetsControllerPaysEffect(damage 2, sacrifice a creature)`: the upkeep's player ("that
/// player") is asked when they can sacrifice a creature; the damage unless they did
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let player = cx.player(0).or(game.active)?;
    let src = cx.source_id();
    let cost = CostDef::SacrificeTargetCost { filter: cx.filters.first()?.clone(), amount: 1 };
    let name = game.obj(src).name().to_string();
    let mut paid = false;
    if can_pay_cost(game, player, src, &cost)
        && choose_use(game, d, player, &format!("Pay sacrifice a creature to prevent ({name} deals 2 damage to that player)?"))?
    {
        paid = pay_cost(game, d, player, src, &cost);
        if d.stopped() {
            return None;
        }
    }
    if !paid {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Player(player)]);
    }
    Some(())
}

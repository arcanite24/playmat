//! Rakdos, Patron of Chaos: "At the beginning of your end step, target opponent may sacrifice two nonland, nontoken
//! permanents. If they don't, you draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RakdosPatronOfChaos.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply)
    .effects(&["new DrawCardSourceControllerEffect(2)"])
    .filters(&["filter"])];

/// Rakdos's `DoUnlessTargetPlayerOrTargetsControllerPaysEffect(draw two, SacrificeTargetCost(2, filter))` (named
/// with the card's class, so other cards' uses aren't this one): when the target player can pay, they're asked
/// (`chooseUse`) and, on a yes, pay; the effect happens unless they paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let cost = CostDef::SacrificeTargetCost { filter: cx.filters[0].clone(), amount: 2 };
    let mut paid = false;
    if can_pay_cost(game, player, cx.source_id(), &cost)
        && choose_use(game, d, player, "Pay sacrifice two nonland, nontoken permanents to prevent (you draw two cards)?")?
    {
        paid = pay_cost(game, d, player, cx.source_id(), &cost);
        if d.stopped() {
            return None;
        }
    }
    if !paid {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}

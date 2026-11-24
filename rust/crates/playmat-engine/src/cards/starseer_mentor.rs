//! Starseer Mentor and Thornplate Intimidator: "target opponent loses 3 life unless they sacrifice a nonland
//! permanent of their choice or discard a card"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("StarseerMentor.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply)
        .effects(&["new LoseLifeTargetEffect(3)"])
        .filters(&["StaticFilters.FILTER_PERMANENT_NON_LAND"]),
    Handler::effect("ThornplateIntimidator.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply)
        .effects(&["new LoseLifeTargetEffect(3)"])
        .filters(&["StaticFilters.FILTER_PERMANENT_NON_LAND"]),
];

/// `DoUnlessTargetPlayerOrTargetsControllerPaysEffect(lose 3 life, OrCost(sacrifice a nonland permanent, discard a
/// card), text)`: when the target player can pay either, they're asked (`chooseUse`) and, on a yes, pay one: with
/// both payable, `OrCost.pay`'s "Sacrifice a nonland permanent or discard a card?" (yes: sacrifice); the life is
/// lost unless they paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let src = cx.source_id();
    let sacrifice = CostDef::SacrificeTargetCost { filter: cx.filters[0].clone(), amount: 1 };
    let discard = CostDef::DiscardCardCost { filter: Filter::default(), amount: 1, different_names: false };
    let usable: Vec<&CostDef> =
        [&sacrifice, &discard].into_iter().filter(|c| can_pay_cost(game, player, src, c)).collect();
    let mut paid = false;
    if !usable.is_empty()
        && choose_use(game, d, player, "Sacrifice a nonland permanent or discard a card to prevent losing 3 life?")?
    {
        let cost = if usable.len() == 2 {
            if choose_use(game, d, player, "Sacrifice a nonland permanent or discard a card?")? {
                usable[0]
            } else {
                usable[1]
            }
        } else {
            usable[0]
        };
        paid = pay_cost(game, d, player, src, cost);
        if d.stopped() {
            return None;
        }
    }
    if !paid {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}

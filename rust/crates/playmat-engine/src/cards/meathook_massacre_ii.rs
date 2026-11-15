//! Meathook Massacre II: "When Meathook Massacre II enters, each player sacrifices X creatures of their choice.
//! Whenever a creature you control dies, you may pay 3 life. If you do, return that card under your control with a
//! finality counter on it. Whenever a creature an opponent controls dies, they may pay 3 life. If they don't, return
//! that card under your control with a finality counter on it."

use super::*;
use crate::card::CostDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("MeathookMassacreII.MeathookMassacreIIEffect", apply).effects(&[
    "new ReturnFromGraveyardToBattlefieldWithCounterTargetEffect(CounterType.FINALITY.createInstance())",
])];

/// the dead creature's controller (as it died) asked "Pay 3 life to prevent this effect?", paying with `PayLifeCost`
/// (`tryPayLife`: not with less life); unpaid, the card (not a token) returns under your control with a finality
/// counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((died, player)) = cx.item.died else { return Some(()) };
    if choose_use(game, d, player, "Pay 3 life to prevent this effect?")?
        && game.player(player).life >= 3
        && pay_cost(game, d, player, cx.source_id(), &CostDef::PayLifeCost { amount: 3 })
    {
        return Some(());
    }
    if d.stopped() {
        return None;
    }
    if game.obj(died).token {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(died)]);
    Some(())
}

//! Unholy Indenture: "Enchant creature. When enchanted creature dies, return that card to the battlefield under your
//! control with a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnholyIndenture.UnholyIndentureReturnEffect", apply)];

/// The creature that died (`getValue("attachedTo")`, the trigger's target), not a token and still the card that went
/// to the graveyard, put onto the battlefield under your control with a +1/+1 counter as it enters
/// (`setEnterWithCounters`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).token || game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: false, counters: None, attacking: false };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 1));
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    Some(())
}

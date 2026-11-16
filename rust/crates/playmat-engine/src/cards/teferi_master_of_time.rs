//! Teferi, Master of Time: "You may activate loyalty abilities of Teferi, Master of Time on any player's turn any
//! time you could cast an instant. +1: Draw a card, then discard a card. −3: Target creature you don't control
//! phases out. −10: Take two extra turns after this one."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::as_though(
        "TeferiMasterOfTime.TeferiMasterOfTimeActivationEffect",
        AsThough::ActivateAsInstant(own_loyalty),
    ),
    Handler::effect("TeferiMasterOfTime.TeferiMasterOfTimeTurnEffect", two_turns),
];

/// this Teferi's own loyalty abilities, activated by its controller
fn own_loyalty(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, index: usize) -> bool {
    id == cx.source
        && seat == cx.controller
        && matches!(game.obj(id).def.abilities.get(index), Some(AbilityDef::LoyaltyAbility { .. }))
}

/// two extra turns for its controller (`TurnMod.withExtraTurn`, twice)
fn two_turns(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::AddExtraTurnControllerEffect;
    cx.run(game, d, &[e.clone(), e]);
    Some(())
}

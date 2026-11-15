//! Prized Amalgam: "Whenever a creature enters, if it entered from your graveyard or you cast it from your
//! graveyard, return Prized Amalgam from your graveyard to the battlefield tapped at the beginning of the next end
//! step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "PrizedAmalgam.PrizedAmalgamTriggerdAbility",
        "new EntersBattlefieldAllTriggeredAbility(Zone.GRAVEYARD, $0, $1, false)",
        from_graveyard,
    ),
    // its effects' target pointer is `FixedTarget(getSourceId(), zcc)`: Prized Amalgam as it is in the graveyard
    Handler::effect("PrizedAmalgam.ReturnFromGraveyardToBattlefieldTargetEffect", apply),
];

/// `checkTrigger`: the creature (one you own, the filter) entered from the graveyard, or from the stack as a spell
/// you control that was cast from the graveyard (`CastFromGraveyardWatcher`)
fn from_graveyard(game: &Game, item: &StackItem) -> bool {
    let Some(it) = item.entering else { return false };
    let o = game.obj(it);
    match o.entered_from {
        Some(Zone::Graveyard) => true,
        Some(Zone::Stack) => o.controller == item.controller && o.cast_from == Some(Zone::Graveyard),
        _ => false,
    }
}

/// `ReturnFromGraveyardToBattlefieldTargetEffect(true)` on Prized Amalgam while it's still that card in the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (o, zcc) = cx.source;
    if game.obj(o).zone != Zone::Graveyard || game.obj(o).zcc != zcc {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::ReturnSourceFromGraveyardToBattlefieldEffect { tapped: true }]);
    Some(())
}

//! Ashnod's Intervention: "Until end of turn, target creature gets +2/+0 and gains "When this creature dies or is
//! put into exile from the battlefield, return it to its owner's hand.""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AshnodsIntervention.AshnodsInterventionAbility",
        "new LeavesBattlefieldTriggeredAbility(new AshnodsInterventionReturnEffect(), false)",
        to_graveyard_or_exile,
    ),
    Handler::effect("AshnodsIntervention.AshnodsInterventionReturnEffect", return_it),
];

/// it left the battlefield for a graveyard or exile (as it triggers; resolving, it holds)
fn to_graveyard_or_exile(game: &Game, t: &StackItem) -> bool {
    game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability)
        || matches!(game.obj(t.card).zone, Zone::Graveyard | Zone::Exile)
}

/// `ReturnToHandTargetEffect` on the card it left as: back to its owner's hand while it's still in that graveyard or
/// exile
fn return_it(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if matches!(game.obj(card).zone, Zone::Graveyard | Zone::Exile) && !game.obj(card).token {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

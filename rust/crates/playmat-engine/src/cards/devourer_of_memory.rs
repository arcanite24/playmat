//! Devourer of Memory: "Whenever one or more cards are put into your graveyard from your library, this creature
//! gets +1/+1 until end of turn and can't be blocked this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "DevourerOfMemory.DevourerOfMemoryTriggeredAbility",
        "new ZoneChangeBatchTriggeredAbility(new DevourerOfMemoryEffect(), Zone.LIBRARY, Zone.GRAVEYARD, StaticFilters.FILTER_CARD, TargetController.YOU, true)",
        any_batch,
    ),
    Handler::effect("DevourerOfMemory.DevourerOfMemoryEffect", apply)
        .effects(&["new BoostSourceEffect(1, 1, Duration.EndOfTurn)", "new CantBeBlockedSourceEffect(Duration.EndOfTurn)"]),
];

/// the class's two effects (its constructor's `addEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, cx.effects);
    Some(())
}

//! Ulamog, the Ceaseless Hunger: "When you cast this spell, exile two target permanents. Indestructible. Whenever
//! Ulamog attacks, defending player exiles the top twenty cards of their library."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "UlamogTheCeaselessHunger.UlamogExilePermanentsOnCastAbility",
        "new CastSourceTriggeredAbility(new ExileTargetEffect(\"exile two target permanents\"))",
        always,
    ),
    Handler::trigger(
        "UlamogTheCeaselessHunger.UlamogAttackTriggeredAbility",
        "new AttacksTriggeredAbility($0, false, null, SetTargetPointer.PLAYER)",
        always,
    ),
    Handler::effect("UlamogTheCeaselessHunger.UlamogExileLibraryEffect", exile_twenty)
        .effects(&["new ExileCardsFromTopOfLibraryTargetEffect(20)"]),
];

/// the base trigger's event is the whole check (`SPELL_CAST` of this card; `ATTACKER_DECLARED` by this, with the
/// defending player as its pointer)
fn always(_: &Game, _: &StackItem) -> bool {
    true
}

/// The defending player (the pointer) moves the top twenty cards of their library to exile
fn exile_twenty(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Player(p)]);
    Some(())
}

//! Double Vision: "Whenever you cast your first instant or sorcery spell each turn, copy that spell. You may choose
//! new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DoubleVision.DoubleVisionCopyTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new CopyTargetStackObjectEffect(false, true, true), new FilterInstantOrSorcerySpell(), false, SetTargetPointer.SPELL)",
    first_instant_or_sorcery,
)];

/// `SpellsCastWatcher`: the spell is the only instant or sorcery its controller cast this turn (the count already
/// has it when cast triggers are checked)
fn first_instant_or_sorcery(game: &Game, t: &StackItem) -> bool {
    game.player(t.controller).instants_sorceries_cast == 1
}

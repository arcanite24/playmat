//! Syr Carah, the Bold: "Whenever Syr Carah, the Bold or an instant or sorcery spell you control deals damage to a
//! player, exile the top card of your library. You may play that card this turn. {T}: Syr Carah, the Bold deals 1
//! damage to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SyrCarahTheBold.SyrCarahTheBoldTriggeredAbility",
    "new DamagedPlayerTriggeredAbility(new ExileTopXMayPlayUntilEffect(1, Duration.EndOfTurn))",
    this_or_your_spell,
)];

/// `DAMAGED_PLAYER` (any player) whose source is this, or an instant or sorcery spell you control
/// (`getSpellOrLKIStack`: the spell may have left the stack by the time the trigger waits, so its card is read)
fn this_or_your_spell(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    let Some(source) = item.entering else { return false };
    if source == item.card {
        return true;
    }
    let o = game.obj(source);
    o.zone != Zone::Battlefield
        && o.controller == item.controller
        && (game.has_type(source, CardType::Instant) || game.has_type(source, CardType::Sorcery))
}

//! Thrashing Frontliner: "Trample. Whenever Thrashing Frontliner attacks a battle, it gets +1/+1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ThrashingFrontliner.ThrashingFrontlinerTriggeredAbility",
    "new AttacksTriggeredAbility(new BoostSourceEffect(1, 1, Duration.EndOfTurn), false)",
    attacks_a_battle,
)];

/// `DECLARED_ATTACKERS` with this attacking (the base), its defender a battle (`getCombat().getDefenderId`), as it
/// triggers (Java checks only then)
fn attacks_a_battle(game: &Game, t: &StackItem) -> bool {
    game.checking_item.is_none()
        || game
            .obj(t.card).attacking_pw.is_some_and(|(o, _)| game.has_type(o, CardType::Battle))
}

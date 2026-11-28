//! Haazda Marshal: "Whenever Haazda Marshal and at least two other creatures attack, create a 1/1 white Soldier
//! creature token with lifelink."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HaazdaMarshal.HaazdaMarshalTriggeredAbility",
    "new AttacksTriggeredAbility(new CreateTokenEffect(new SoldierLifelinkToken()), false)",
    check,
)];

/// `DECLARED_ATTACKERS` with three or more attackers, this among them (it's this one's attack trigger). Java checks
/// it only as it triggers: as it resolves it holds
fn check(game: &Game, t: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability) {
        return true;
    }
    game.battlefield.iter().filter(|o| game.obj(**o).attacking.is_some()).count() >= 3
}

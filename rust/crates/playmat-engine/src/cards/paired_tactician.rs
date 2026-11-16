//! Paired Tactician: "Whenever Paired Tactician and at least one other Warrior attack, put a +1/+1 counter on
//! Paired Tactician."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "PairedTactician.PairedTacticianTriggeredAbility",
    "new AttacksWithCreaturesTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), 2)",
    check,
)];

/// `DECLARED_ATTACKERS`: it's among the attackers, and so is another Warrior
fn check(game: &Game, t: &StackItem) -> bool {
    let me = t.card;
    let attacking = |id: ObjId| game.obj(id).attacking.is_some() || game.obj(id).attacking_pw.is_some();
    game.obj(me).zone == Zone::Battlefield
        && attacking(me)
        && game.battlefield.iter().any(|o| *o != me && attacking(*o) && game.has_subtype(*o, "Warrior"))
}

//! Skull Skaab: "Exploit. Whenever a creature you control exploits a nontoken creature, create a 2/2 black Zombie
//! creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SkullSkaab.SkullSkaabTriggeredAbility",
    "new ExploitedCreatureAnyTriggeredAbility(new CreateTokenEffect(new ZombieToken()))",
    yours_exploits_nontoken,
)];

/// `EXPLOITED_CREATURE`: the exploiter (as it last was) a creature you control, the exploited (as it last was) a
/// creature that isn't a token. Checked as it triggers only.
fn yours_exploits_nontoken(game: &Game, item: &StackItem) -> bool {
    let same = |r: &StackItem| r.card == item.card && r.ability == item.ability;
    if game.checking_item.as_ref().is_some_and(same) || game.resolving_item.as_ref().is_some_and(same) {
        return true;
    }
    let (Some(exploiter), Some(exploited)) = (item.entering, item.sacrificed_permanent) else { return false };
    game.has_type(exploiter, CardType::Creature)
        && game.obj(exploiter).controller == item.controller
        && game.has_type(exploited, CardType::Creature)
        && !game.obj(exploited).token
}

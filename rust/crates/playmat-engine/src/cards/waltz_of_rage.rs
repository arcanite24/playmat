//! Waltz of Rage: "Target creature you control deals damage equal to its power to each other creature. Until end of
//! turn, whenever a creature you control dies, exile the top card of your library. You may play it until the end of
//! your next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WaltzOfRage.WaltzOfRageEffect", apply),
    Handler::trigger(
        "WaltzOfRage.WaltzOfRageTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new ExileTopXMayPlayUntilEffect(1, Duration.UntilEndOfYourNextTurn), false, StaticFilters.FILTER_CONTROLLED_CREATURE)",
        any,
    ),
];

/// The target deals damage equal to its power (at least 1) to each other creature, one at a time in battlefield
/// order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(me) = cx.obj(0) else { return Some(()) };
    if game.obj(me).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(me).0;
    if power < 1 {
        return Some(());
    }
    let others: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|c| *c != me && game.has_type(*c, CardType::Creature)).collect();
    for c in others {
        if game.obj(c).zone == Zone::Battlefield {
            deal_damage(game, me, Target::Obj(c), power, false);
        }
    }
    Some(())
}

/// The base trigger says all of it (a dies event of a creature you control)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}

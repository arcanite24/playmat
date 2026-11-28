//! Tolsimir, Friend to Wolves: "Whenever a Wolf you control enters, you gain 3 life and that creature fights up to
//! one target creature an opponent controls."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TolsimirFriendToWolves.TolsimirFriendToWolvesTriggeredAbility",
        r#"new EntersBattlefieldAllTriggeredAbility(new TolsimirFriendToWolvesEffect(), new FilterControlledPermanent(SubType.WOLF, "a Wolf you control"))"#,
        any,
    ),
    Handler::effect("TolsimirFriendToWolves.TolsimirFriendToWolvesEffect", apply),
];

fn any(_: &Game, _: &StackItem) -> bool {
    true
}

/// you gain 3 life; then the Wolf that entered (still that permanent) fights the target, if there is one
/// (`Permanent.fight`: the target deals its damage first, then the Wolf only what's lethal)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    gain_life(game, cx.controller(), 3);
    let Some(target) = cx.obj(0) else { return Some(()) };
    let Some(wolf) = cx.item.entering else { return Some(()) };
    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
    if creature(game, wolf) && creature(game, target) {
        let pt = game.power_toughness(target).0;
        let pw = game.power_toughness(wolf).0;
        deal_damage(game, target, Target::Obj(wolf), pt, false);
        let pw = pw.min(fight_lethal(game, target, wolf));
        deal_damage(game, wolf, Target::Obj(target), pw, false);
    }
    Some(())
}

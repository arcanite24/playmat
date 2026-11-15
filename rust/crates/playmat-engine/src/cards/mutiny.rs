//! Mutiny: "Target creature an opponent controls deals damage equal to its power to another target creature that
//! player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "Mutiny.MutinySecondTarget",
        "new TargetPermanent(StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE)",
        same_controller,
    ),
    Handler::effect("Mutiny.MutinyEffect", apply),
];

/// `possibleTargets`: with the first target chosen, only another creature its controller controls; before it's
/// chosen (the playable check), any, unless it would be the only one ("workaround to make 1 target invalid"); checked
/// as the spell resolves, only the base filter (`canTarget` isn't overridden)
fn same_controller(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Obj(first)) if *first != id => {
            game.obj(*first).zone != Zone::Battlefield || game.obj(*first).controller == game.obj(id).controller
        }
        Some(Target::Obj(_)) => false,
        _ => {
            game.resolving_item.is_some()
                || game
                    .battlefield
                .iter()
                .filter(|o| game.obj(**o).controller != you && game.has_type(**o, CardType::Creature))
                .count()
                > 1
        }
    }
}

/// The first target, still on the battlefield, deals damage equal to its power to the second target's permanent
/// (Java's `source.getTargets().get(1).getFirstTarget()`: still on the battlefield), with itself as the source
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(first) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let damage = game.power_toughness(first).0;
    let Some(second) = cx.item.targets.get(1).and_then(|ts| ts.first()).and_then(|t| match t {
        Target::Obj(o) => Some(*o),
        _ => None,
    }) else {
        return Some(());
    };
    let zcc = cx.item.target_zcc.get(1).and_then(|z| z.first()).copied();
    if damage > 0 && game.obj(second).zone == Zone::Battlefield && Some(game.obj(second).zcc) == zcc {
        deal_damage(game, first, Target::Obj(second), damage, false);
    }
    Some(())
}

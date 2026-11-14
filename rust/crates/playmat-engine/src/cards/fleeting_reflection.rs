//! Fleeting Reflection: "Target creature you control gains hexproof until end of turn. Untap that creature. Until end
//! of turn, it becomes a copy of up to one other target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FleetingReflection.FleetingReflectionEffect", apply)];

/// `game.copyPermanent(Duration.EndOfTurn, copyFrom, copyTo, …)`: the first target still there becomes a copy of the
/// second (or of it as it last was on the battlefield), when one was chosen
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let to = cx.obj(0)?;
    let from = match cx.item.targets.get(1).and_then(|t| t.first()) {
        Some(Target::Obj(o)) => *o,
        _ => return Some(()),
    };
    if game.obj(to).zone != Zone::Battlefield
        || (game.obj(from).zone != Zone::Battlefield && !game.short_lki.contains(&from))
    {
        return Some(());
    }
    let copy = game.obj(from).copiable();
    become_copy_until_end_of_turn(game, to, copy);
    Some(())
}

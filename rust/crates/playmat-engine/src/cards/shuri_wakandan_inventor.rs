//! Shuri, Wakandan Inventor: "Artifact spells you cast cost {1} less to cast. {1}, {T}: Target artifact you control
//! becomes a copy of a second target artifact you control until end of turn, except it isn't legendary. Activate
//! only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShuriWakandanInventor.ShuriWakandanInventorEffect", apply)];

/// `game.copyPermanent(Duration.EndOfTurn, copyFrom, copyTo, …)` with a `CopyApplier` removing legendary: the first
/// target still there becomes a copy of the second (or of it as it last was on the battlefield)
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
    let mut copy = game.obj(from).copiable();
    std::sync::Arc::make_mut(&mut copy).supertypes.retain(|s| s != "LEGENDARY");
    become_copy_until_end_of_turn(game, to, copy);
    Some(())
}

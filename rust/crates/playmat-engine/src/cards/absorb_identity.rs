//! Absorb Identity: "Return target creature to its owner's hand. You may have Shapeshifters you control become copies
//! of that creature until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AbsorbIdentity.AbsorbIdentityEffect", apply)];

/// The target as it was (`getPermanentOrLKIBattlefield`, it's been returned by now): "Have Shapeshifters you control
/// become copies of <it> until end of turn?"; yes: each Shapeshifter creature you control, in battlefield order,
/// becomes a copy of it until end of turn (`game.copyPermanent(Duration.EndOfTurn, …, EmptyCopyApplier)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let from = match cx.item.targets.first().and_then(|t| t.first()) {
        Some(Target::Obj(o)) => *o,
        _ => return Some(()),
    };
    let message = format!(
        "Have Shapeshifters you control become copies of {} until end of turn?",
        log::object(game, from)
    );
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    let shapeshifters: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
                && game.has_subtype(*o, "Shapeshifter")
        })
        .collect();
    let copy = game.obj(from).copiable();
    for to in shapeshifters {
        become_copy_until_end_of_turn(game, to, copy.clone());
    }
    Some(())
}

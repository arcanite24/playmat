//! Saheeli, Sublime Artificer: "−2: Target artifact you control becomes a copy of another target artifact or creature
//! you control until end of turn, except it's an artifact in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SaheeliSublimeArtificer.SaheeliSublimeArtificerEffect", apply)];

/// `game.copyPermanent(EndOfTurn, …, EmptyCopyApplier)` from the second target (as it last was) onto the first, and
/// `AddCardTypeTargetEffect(EndOfTurn, ARTIFACT)` on it: both end with the turn, so the artifact type goes with the
/// copied values
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let to = cx.obj(0)?;
    let from = cx.item.targets.get(1)?.iter().find_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None })?;
    if game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    let mut copy = (*game.obj(from).copiable()).clone();
    if !copy.types.contains(&CardType::Artifact) {
        copy.types.push(CardType::Artifact);
    }
    become_copy_until_end_of_turn(game, to, Arc::new(copy));
    Some(())
}

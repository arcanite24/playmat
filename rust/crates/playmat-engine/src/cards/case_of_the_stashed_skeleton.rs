//! Case of the Stashed Skeleton: "When this Case enters the battlefield, create a 2/1 black Skeleton creature token
//! and suspect it. To solve — You control no suspected Skeletons. Solved — {1}{B}, Sacrifice this Case: Search your
//! library for a card, put it into your hand, then shuffle. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CaseOfTheStashedSkeleton.CaseOfTheStashedSkeletonEffect", apply)
    .tokens(&["Skeleton21Token"])];

/// `CreateTokenEffect(Skeleton21Token)`, then the first token it made (`getLastAddedTokenIds`) is suspected
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let first = game.objs.len();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    if d.stopped() {
        return None;
    }
    let made = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .find(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield);
    if let Some(o) = made {
        game.obj_mut(o).set_suspected(true);
    }
    Some(())
}

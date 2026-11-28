//! Second Harvest: "For each token you control, create a token that's a copy of that permanent."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SecondHarvest.SecondHarvestEffect", apply).effects(&["new CreateTokenCopyTargetEffect()"])];

/// The tokens you control as it begins (`getAllActivePermanents`, in battlefield order), each copied in turn by a
/// `CreateTokenCopyTargetEffect` pointed at it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let tokens: Vec<(ObjId, u32)> = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).token && game.obj(**o).controller == you)
        .map(|o| (*o, game.obj(*o).zcc))
        .collect();
    for (t, _) in tokens {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(t)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

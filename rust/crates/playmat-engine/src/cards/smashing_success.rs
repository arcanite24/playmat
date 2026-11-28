//! Smashing Success: "Destroy target artifact or land. If an artifact is destroyed this way, create a Treasure
//! token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SmashingSuccess.SmashingSuccessEffect", apply).tokens(&["TreasureToken"])];

/// An artifact that leaves the battlefield as it's destroyed makes the Treasure
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let artifact = game.has_type(o, CardType::Artifact);
    let zcc = game.obj(o).zcc;
    destroy(game, o);
    if artifact && (game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc) {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

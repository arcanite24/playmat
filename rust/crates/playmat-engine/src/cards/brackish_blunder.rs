//! Brackish Blunder: "Return target creature to its owner's hand. If it was tapped, create a Map token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrackishBlunder.BrackishBlunderEffect", apply)
    .effects(&["new CreateTokenEffect(new MapToken())"])];

/// The target, if still legal, to its owner's hand; a Map token for you if it was tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let tapped = game.obj(o).tapped;
    game.move_to(o, Zone::Hand);
    if tapped {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

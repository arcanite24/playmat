//! Break the Spell: "Destroy target enchantment. If a permanent you controlled or a token was destroyed this way,
//! draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BreakTheSpell.BreakTheSpellEffect", apply)];

/// whether it's yours or a token, read before the destroy; the draw only when it left the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let followup = game.obj(o).controller == cx.controller() || game.obj(o).token;
    let zcc = game.obj(o).zcc;
    cx.run(game, d, &[EffectDef::DestroyTargetEffect]);
    if d.stopped() {
        return None;
    }
    let destroyed = game.obj(o).zcc != zcc || game.obj(o).zone != Zone::Battlefield;
    if followup && destroyed {
        draw(game, cx.controller(), 1);
    }
    Some(())
}

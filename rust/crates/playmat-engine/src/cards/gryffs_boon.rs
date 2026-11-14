//! Gryff's Boon: "{3}{W}: Return Gryff's Boon from your graveyard to the battlefield attached to target creature.
//! Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GryffsBoon.GryffsBoonEffect", apply)];

/// Still in the graveyard: it enters attached to the target (`"attachTo:" + auraId`), not asked where
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(aura).zone != Zone::Graveyard || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    game.attach_entering = Some((aura, to, game.obj(to).zcc));
    put_onto_battlefield(game, d, aura, cx.controller());
    game.attach_entering = None;
    if d.stopped() {
        return None;
    }
    Some(())
}

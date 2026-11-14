//! Dramatic Accusation: "{U}{U}: Shuffle enchanted creature into its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DramaticAccusation.DramaticAccusationEffect", apply)];

/// What the Aura is (or was last) attached to, into its owner's library, and that owner's shuffle
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    let (o, zcc) = if src.zcc == cx.source.1 { src.attached_to } else { src.lki_attached_to }?;
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return None;
    }
    let owner = game.obj(o).owner;
    to_library(game, o, true);
    let mut library = game.player(owner).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(owner).library = library;
    Some(())
}

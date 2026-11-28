//! Stay Hidden, Stay Silent: "{4}{U}{U}: Shuffle enchanted creature into its owner's library, then manifest dread."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StayHiddenStaySilent.StayHiddenStaySilentEffect", apply)];

/// The permanent the Aura is (or, gone, was last) attached to, while it's still on the battlefield, into its
/// owner's library, which is shuffled (`shuffleCardsToLibrary`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = game.obj(cx.source_id());
    let (c, zcc) = if cx.source_on_battlefield(game) { aura.attached_to } else { aura.lki_attached_to }?;
    if game.obj(c).zone != Zone::Battlefield || game.obj(c).zcc != zcc {
        return Some(());
    }
    let owner = game.obj(c).owner;
    to_library(game, c, true);
    let mut library = game.player(owner).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(owner).library = library;
    Some(())
}

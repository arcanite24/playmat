//! Bookwurm: "{2}{G}: Put this card from your graveyard into your library third from the top."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Bookwurm.BookwurmEffect", apply)];

/// `putCardOnTopXOfLibrary(card, 3)`: third from the top, or the bottom of a library of fewer than two cards
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (o, zcc) = cx.source;
    if game.obj(o).zone != Zone::Graveyard || game.obj(o).zcc != zcc {
        return Some(());
    }
    let owner = game.obj(o).owner;
    let size = game.player(owner).library.len();
    game.move_to(o, Zone::Library);
    let lib = &mut game.player_mut(owner).library;
    lib.retain(|x| *x != o);
    if size + 1 < 3 {
        lib.push(o);
    } else {
        lib.insert(2, o);
    }
    Some(())
}

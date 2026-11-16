//! Commit // Memory: "Put target spell or nonland permanent into its owner's library second from the top." //
//! "Aftermath. Each player shuffles their hand and graveyard into their library, then draws seven cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CommitMemory.CommitEffect", apply)];

/// `CommitEffect`: a permanent target first, else a spell (`putCardOnTopXOfLibrary(…, 2, true)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    match game.obj(o).zone {
        Zone::Battlefield => cx.run(game, d, &[EffectDef::PutIntoLibraryNFromTopTargetEffect { n: 2 }]),
        Zone::Stack => {
            let owner = game.obj(o).owner;
            let size = game.player(owner).library.len();
            game.move_to(o, Zone::Library);
            // (a library with nothing in it: the bottom, which is the top)
            if size + 1 >= 2 && game.obj(o).zone == Zone::Library && !game.obj(o).token {
                let lib = &mut game.player_mut(owner).library;
                lib.retain(|x| *x != o);
                lib.insert(1.min(lib.len()), o);
            }
        }
        _ => {}
    }
    Some(())
}

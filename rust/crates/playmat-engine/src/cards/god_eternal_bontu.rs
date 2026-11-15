//! God-Eternal Bontu: "When God-Eternal Bontu enters, sacrifice any number of other permanents, then draw that many
//! cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GodEternalBontu.GodEternalBontuEffect", apply)];

/// `TargetSacrifice(0, MAX, other permanents you control)` chosen with `Player.choose`, each sacrificed in the order
/// chosen, then a card drawn for each one sacrificed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let can: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|p| *p != src && game.obj(*p).controller == you).collect();
    let chosen = choose_cards(game, d, you, &can, &can, 0, usize::MAX, "sacrifice")?;
    let mut n = 0;
    for p in chosen {
        if game.obj(p).zone == Zone::Battlefield && game.obj(p).controller == you {
            sacrifice(game, p);
            n += 1;
        }
    }
    draw(game, you, n);
    Some(())
}

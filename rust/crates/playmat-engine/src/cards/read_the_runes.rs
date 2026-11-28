//! Read the Runes: "Draw X cards. For each card drawn this way, discard a card unless you sacrifice a permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReadTheRunes.ReadTheRunesEffect", apply)];

/// Draw X (`getSourceCostsTag "X"`); then up to that many of your permanents are chosen (`TargetSacrifice(0, n)`,
/// `Player.choose`) and sacrificed in that order; then the rest is discarded
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.player(you).library.len();
    draw(game, you, cx.item.x.max(0) as usize);
    let drawn = before.saturating_sub(game.player(you).library.len());
    let possible: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
    let chosen = if drawn == 0 || possible.is_empty() {
        Vec::new()
    } else {
        choose_cards_rounds(game, d, you, &possible, 0, drawn)?
    };
    let mut sacrificed = 0;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield && game.obj(o).controller == you {
            sacrifice(game, o);
            sacrificed += 1;
        }
    }
    let rest = drawn as i32 - sacrificed;
    if rest > 0 {
        discard(game, d, you, rest);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

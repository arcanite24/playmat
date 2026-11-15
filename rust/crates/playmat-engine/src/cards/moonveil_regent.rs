//! Moonveil Regent: "Flying. Whenever you cast a spell, you may discard your hand. If you do, draw a card for each
//! of that spell's colors. When Moonveil Regent dies, it deals X damage to any target, where X is the number of
//! colors among permanents you control."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::value("MoonveilRegent.MoonveilRegentSpellValue", spell_colors),
    Handler::value("MoonveilRegent.MoonveilRegentColorValue", colors_among_yours),
];

const COLORS: [Color; 5] = [Color::W, Color::U, Color::B, Color::R, Color::G];

/// the colors of the spell the trigger saw (`getValue("spellCast")`); 0 without one
fn spell_colors(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let item = game.resolving_item.as_ref().or(game.checking_item.as_ref());
    item.and_then(|i| i.cast_spell).map_or(0, |s| {
        let colors = game.colors(s);
        COLORS.iter().filter(|c| colors.contains(c)).count() as i32
    })
}

/// the colors among permanents you control
fn colors_among_yours(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mine: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
    COLORS.iter().filter(|c| mine.iter().any(|o| game.colors(*o).contains(c))).count() as i32
}

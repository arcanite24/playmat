//! Malamet Battle Glyph: "Choose target creature you control and target creature you don't control. If the creature
//! you control entered the battlefield this turn, put a +1/+1 counter on it. Then those creatures fight each other."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MalametBattleGlyph.MalametBattleGlyphEffect", apply)];

/// `getTurnsOnBattlefield() == 0`: it entered this turn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).entered_turn == game.turn {
        put_counters(game, o, "+1/+1", 1, cx.controller());
    }
    Some(())
}

//! Corrupt: "Corrupt deals damage to any target equal to the number of Swamps you control. You gain life equal to
//! the damage dealt this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Corrupt.CorruptEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land) && game.has_subtype(**o, "Swamp"))
        .count() as i32;
    if n < 1 {
        return Some(());
    }
    let dealt = if let Some(o) = cx.obj(0) {
        deal_damage(game, cx.source_id(), Target::Obj(o), n, false)
    } else if let Some(p) = cx.player(0) {
        deal_damage(game, cx.source_id(), Target::Player(p), n, false)
    } else {
        return Some(());
    };
    gain_life(game, you, dealt);
    Some(())
}

//! Dire Tactics: "Exile target creature. If you don't control a Human, you lose life equal to that creature's
//! toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DireTactics.DireTacticsEffect", apply)];

/// The toughness is read before the exile, whether you control a Human after it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let toughness = game.power_toughness(t).1;
    game.move_to(t, Zone::Exile);
    let human = game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Human"));
    if !human && toughness > 0 {
        lose_life(game, you, toughness);
    }
    Some(())
}

//! Devour Flesh: "Target player sacrifices a creature, then gains life equal to that creature's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DevourFlesh.DevourFleshSacrificeEffect", apply)];

/// `TargetSacrifice` of one creature (picked for them when only one fits), its toughness, the sacrifice, the life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    if creatures.is_empty() {
        return Some(());
    }
    let &o = choose_cards(game, d, p, &creatures, &creatures, 1, 1, "sacrifice")?.first()?;
    let toughness = game.power_toughness(o).1;
    sacrifice(game, o);
    gain_life(game, p, toughness);
    Some(())
}

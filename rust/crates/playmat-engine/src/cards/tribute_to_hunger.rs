//! Tribute to Hunger: "Target opponent sacrifices a creature of their choice. You gain life equal to that creature's
//! toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TributeToHunger.TributeToHungerEffect", apply)];

/// the opponent chooses (`TargetSacrifice(FILTER_PERMANENT_CREATURE)`), its toughness read before it's sacrificed
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
    let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, "sacrifice")?;
    let Some(o) = chosen.first().copied() else { return Some(()) };
    let toughness = game.power_toughness(o).1;
    sacrifice(game, o);
    gain_life(game, cx.controller(), toughness);
    Some(())
}

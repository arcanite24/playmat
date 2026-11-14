//! Ritual of Hope: "Creatures you control get +1/+1 until end of turn. Coven — If you control three or more creatures
//! with different powers, creatures you control get +2/+1 until end of turn instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RitualOfHope.RitualOfHopeEffect", apply).effects(&[
    "new BoostControlledEffect(2, 1, Duration.EndOfTurn)",
    "new BoostControlledEffect(1, 1, Duration.EndOfTurn)",
])];

/// Coven (`CovenCondition`) as this resolves picks the boost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut powers: Vec<i32> = Vec::new();
    for &o in game.battlefield.iter() {
        if game.obj(o).controller == you && game.has_type(o, CardType::Creature) {
            let p = game.power_toughness(o).0;
            if !powers.contains(&p) {
                powers.push(p);
            }
        }
    }
    let i = if powers.len() >= 3 { 0 } else { 1 };
    cx.run(game, d, &cx.effects[i..=i]);
    Some(())
}

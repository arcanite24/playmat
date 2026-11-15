//! Flames of the Raze-Boar: "This spell deals 4 damage to target creature an opponent controls. Then this spell
//! deals 2 damage to each other creature that player controls if you control a creature with power 4 or greater."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FlamesOfTheRazeBoar.FlamesOfTheRazeBoarEffect", apply)];

/// The 4 damage, then `FerociousCondition` as it is after it, then `DamageAllEffect` over the creatures of the
/// target's controller other than it (one at a time, in battlefield order)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let src = cx.source_id();
    let p = game.obj(o).controller;
    deal_damage(game, src, Target::Obj(o), 4, false);
    let you = cx.controller();
    let ferocious = game.battlefield.iter().any(|b| {
        game.obj(*b).controller == you && game.has_type(*b, CardType::Creature) && game.power_toughness(*b).0 >= 4
    });
    if !ferocious {
        return Some(());
    }
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| *b != o && game.obj(*b).controller == p && game.has_type(*b, CardType::Creature))
        .collect();
    for c in others {
        deal_damage(game, src, Target::Obj(c), 2, false);
    }
    Some(())
}

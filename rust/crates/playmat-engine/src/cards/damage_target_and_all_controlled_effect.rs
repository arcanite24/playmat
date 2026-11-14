//! `DamageTargetAndAllControlledEffect` (Chandra's Fury, Chandra, Flame's Fury): "deals 4 damage to target player or
//! planeswalker and 1 damage to each creature that player or that planeswalker's controller controls".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageTargetAndAllControlledEffect", apply)];

/// `(amount)` or `(first, second, FILTER_PERMANENT_CREATURE)`: the target first, then each creature of the player
/// (or the planeswalker's controller), in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let first = cx.arg_int(0)?;
    let second = if cx.args.len() >= 2 { cx.arg_int(1)? } else { first };
    if cx.args.len() == 3 && cx.args[2].trim() != "StaticFilters.FILTER_PERMANENT_CREATURE" {
        return Some(());
    }
    let src = cx.source_id();
    let player = match cx.target(0)? {
        Target::Obj(o) => {
            let p = game.obj(o).controller;
            if game.obj(o).zone == Zone::Battlefield {
                deal_damage(game, src, Target::Obj(o), first, false);
            }
            p
        }
        Target::Player(p) => {
            deal_damage(game, src, Target::Player(p), first, false);
            p
        }
    };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == player && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        if game.obj(o).zone == Zone::Battlefield {
            deal_damage(game, src, Target::Obj(o), second, false);
        }
    }
    Some(())
}

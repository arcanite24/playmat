//! Judgment Bolt: "Judgment Bolt deals 5 damage to target creature and X damage to that creature's controller, where
//! X is the number of Equipment you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JudgmentBolt.JudgmentBoltEffect", apply)];

/// `DamageTargetAndTargetControllerEffect(5, X)`: X counted first, the controller as the creature last was
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let x = game.battlefield.iter().filter(|p| game.obj(**p).controller == you && game.has_subtype(**p, "Equipment")).count();
    let player = game.obj(o).controller;
    deal_damage(game, cx.source_id(), Target::Obj(o), 5, false);
    if x > 0 {
        deal_damage(game, cx.source_id(), Target::Player(player), x as i32, false);
    }
    Some(())
}

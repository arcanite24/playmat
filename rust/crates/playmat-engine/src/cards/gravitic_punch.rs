//! Gravitic Punch: "Target creature you control deals damage equal to its power to target player. Jump-start"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GraviticPunch.GraviticPunchEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(o), Some(p)) = (cx.obj(0), cx.player(1)) else { return Some(()) };
    let (power, _) = game.power_toughness(o);
    deal_damage(game, o, Target::Player(p), power, false);
    Some(())
}

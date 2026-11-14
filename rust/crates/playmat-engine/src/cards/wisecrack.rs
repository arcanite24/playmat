//! Wisecrack: "Target creature deals damage equal to its power to itself. If that creature is attacking, Wisecrack
//! deals 2 damage to that creature's controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Wisecrack.WisecrackEffect", apply)];

/// The creature's damage to itself, then the spell's 2 to its controller while it's attacking
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let power = game.power_toughness(o).0;
    deal_damage(game, o, Target::Obj(o), power, false);
    if game.obj(o).attacking.is_some() {
        let p = game.obj(o).controller;
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}

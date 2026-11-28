//! The Last Agni Kai: "Target creature you control fights target creature an opponent controls. If the creature the
//! opponent controls is dealt excess damage this way, add that much {R}. Until end of turn, you don't lose this mana
//! as steps and phases end."

use super::damage_with_excess_effect::damage_with_excess;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheLastAgniKai.TheLastAgniKaiEffect", apply)];

/// `fightWithExcess`: the opponent's creature deals its damage first, then yours deals up to lethal damage to it
/// (`damageWithExcess`); the rest is added as {R}
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(a), Some(b)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    let here = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield;
    if !here(game, a) || !here(game, b) {
        return Some(());
    }
    let pb = game.power_toughness(b).0;
    let pa = game.power_toughness(a).0;
    deal_damage(game, b, Target::Obj(a), pb, false);
    let excess = damage_with_excess(game, a, b, pa);
    if excess > 0 {
        game.player_mut(cx.controller()).mana_pool.colored[Color::R.index()] += excess as u32;
    }
    Some(())
}

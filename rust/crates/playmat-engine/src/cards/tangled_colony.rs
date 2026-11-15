//! Tangled Colony: "Tangled Colony can't block. When Tangled Colony dies, create X 1/1 black Rat creature tokens with
//! "This creature can't block," where X is the amount of damage dealt to it this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("TangledColony.TangledColonyDynamicValue", damage_to_it)];

/// `TangledColonyWatcher`: the damage dealt this turn to the permanent the source is (or, once it's left the
/// battlefield, was: `getPermanentOrLKIBattlefield`)
fn damage_to_it(game: &Game, _: Seat, source: ObjId, _: i32) -> i32 {
    let o = game.obj(source);
    let zcc = if o.zone == Zone::Battlefield { o.zcc } else { o.zcc.saturating_sub(1) };
    game.watch
        .damage
        .iter()
        .filter(|s| s.to == Target::Obj(source) && s.to_zcc == zcc)
        .map(|s| s.amount)
        .sum()
}

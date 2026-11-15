//! Wolverine, Fierce Fighter: "Haste. When Wolverine enters, he fights up to one other target creature. If damage
//! would be dealt to Wolverine, instead that damage is dealt, but all other damage already dealt to him is healed."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "WolverineFierceFighter.WolverineFierceFighterEffect",
    &[EventType::DamagePermanent],
    to_wolverine,
    heal_first,
)];

/// damage to Wolverine himself
fn to_wolverine(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source)
}

/// `removeAllDamage` (and the deathtouch mark), then the same damage is dealt (this replacement applied to it)
fn heal_first(game: &mut Game, cx: &StaticCx, _: &mut Event) -> bool {
    let o = game.obj_mut(cx.source);
    o.damage = 0;
    o.deathtouched = false;
    false
}

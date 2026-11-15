//! Ancient Adamantoise: "Vigilance. Ward {3}. Damage isn't removed from this creature during cleanup steps. All
//! damage that would be dealt to you and other permanents you control is dealt to this creature instead. When this
//! creature dies, exile it and create ten tapped Treasure tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::rule("AncientAdamantoise.AncientAdamantoiseDamageEffect", Rule::KeepsDamage(keeps)),
    Handler::replacement(
        "AncientAdamantoise.AncientAdamantoiseRedirectEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        applies,
        replace,
    ),
];

/// `REMOVE_DAMAGE_EOT` of this permanent
fn keeps(_game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// damage to you, or to another permanent you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| t != cx.source && game.obj(t).controller == cx.controller),
    }
}

/// dealt to this creature instead (`permanent.damage(…)` with the event's source, combat and preventable flags)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone == Zone::Battlefield {
        redirect_damage(game, ev, cx.source);
    }
    true
}

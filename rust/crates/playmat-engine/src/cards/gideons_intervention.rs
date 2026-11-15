//! Gideon's Intervention: "Your opponents can't cast spells with the chosen name." and "Prevent all damage that
//! would be dealt to you and permanents you control by sources with the chosen name."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::rule("GideonsIntervention.GideonsInterventionCantCastEffect", Rule::CantCast(cant_cast)),
    Handler::replacement(
        "GideonsIntervention.GideonsInterventionPreventAllDamageEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        applies,
        replace,
    ),
];

/// the name `ChooseACardNameEffect` kept on it
fn chosen(game: &Game, cx: &StaticCx) -> Option<String> {
    game.obj(cx.source).chosen_name.clone().filter(|n| !n.is_empty())
}

/// an opponent casting a spell (its characteristics as cast) with the chosen name
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, face: &CardDef) -> bool {
    cx.opponent(seat) && chosen(game, cx).is_some_and(|n| face.name == n)
}

/// `PreventionEffectImpl.applies`: damage from a source with the chosen name to you or a permanent you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    let Some(name) = chosen(game, cx) else { return false };
    if game.obj(s).name() != name {
        return false;
    }
    let to_you = match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        EventType::DamagePermanent => ev.target.is_some_and(|t| game.obj(t).controller == cx.controller),
        _ => false,
    };
    to_you && cx.preventable(ev, false)
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}

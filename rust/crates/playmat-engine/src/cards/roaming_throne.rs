//! Roaming Throne: "As Roaming Throne enters, choose a creature type. Roaming Throne is the chosen type in addition
//! to its other types. If a triggered ability of another creature you control of the chosen type triggers, it
//! triggers an additional time."
//!
//! Also the framework's `AddChosenSubtypeEffect` (layer 4: the source is its chosen creature type) and
//! `EnterAttributeAddChosenSubtypeEffect` (the same while it enters, which the layer 4 effect already answers here).

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "RoamingThrone.RoamingThroneReplacementEffect",
        &[EventType::NumberOfTriggers],
        applies,
        replace,
    ),
    Handler::continuous("AddChosenSubtypeEffect", is_source, Layer::Type(chosen_type)),
    Handler::effect("EnterAttributeAddChosenSubtypeEffect", entering),
];

/// the trigger is yours, of another creature you control that has the chosen type
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    let Some(t) = game.obj(cx.source).chosen_type.as_ref() else { return false };
    let o = game.obj(src);
    ev.player == cx.controller
        && src != cx.source
        && o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && game.has_type(src, CardType::Creature)
        && game.has_subtype(src, t)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}

fn is_source(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

fn chosen_type(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<String> {
    game.obj(cx.source).chosen_type.iter().cloned().collect()
}

/// the static `AddChosenSubtypeEffect` gives the type from the moment the permanent is on the battlefield
fn entering(_: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    Some(())
}

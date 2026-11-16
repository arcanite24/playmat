//! Stonewise Fortifier: "{4}{W}: Prevent all damage that would be dealt to this creature by target creature this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "StonewiseFortifier.StonewiseFortifierPreventAllDamageToEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage to the source from the ability's target creature
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false)
        && ev.target == Some(cx.source)
        && ev.source.is_some()
        && cx.target_obj(game, 0) == ev.source
}

fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}

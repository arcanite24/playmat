//! Silburlind Snapper: "Silburlind Snapper can't attack unless you've cast a noncreature spell this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "SilburlindSnapper.SilburlindSnapperEffect",
    applies,
    &[Restrict::CantAttack(cant_attack)],
)];

/// the Snapper itself, while you haven't cast a noncreature spell this turn (`SpellsCastWatcher`)
fn applies(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && !game.watch.spells.iter().any(|s| s.caster == cx.controller && !s.creature)
}

fn cant_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

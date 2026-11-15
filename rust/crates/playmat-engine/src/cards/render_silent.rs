//! Render Silent: "Counter target spell. Its controller can't cast spells this turn."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("RenderSilent.RenderSilentCounterEffect", counter).effects(&["new CounterTargetEffect()"]),
    Handler::rule("RenderSilent.RenderSilentEffect", Rule::CantCast(cant)),
];

fn counter(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    Some(())
}

/// the countered spell's controller (the target pointer Java moves to that player as the spell is countered)
fn cant(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, _: &CardDef) -> bool {
    match cx.targets.first() {
        Some((Target::Obj(o), _)) => game.obj(*o).controller == seat,
        Some((Target::Player(p), _)) => *p == seat,
        None => false,
    }
}

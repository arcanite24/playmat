//! Mindswipe: "Counter target spell unless its controller pays {X}. Mindswipe deals X damage to that spell's
//! controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Mindswipe.MindswipeEffect", apply)];

/// X damage to the spell's controller (the one on the stack, else its last known controller), countered or not
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.item.targets.first()?.first().and_then(|t| if let Target::Obj(o) = t { Some(*o) } else { None })?;
    let p = game.stack.iter().find(|i| i.card == o && i.ability.is_none()).map(|i| i.controller).unwrap_or(game.obj(o).controller);
    deal_damage(game, cx.source_id(), Target::Player(p), cx.item.x, false);
    Some(())
}

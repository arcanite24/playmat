//! Raffine, Scheming Seer: "Whenever you attack, target attacking creature connives X, where X is the number of
//! attacking creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RaffineSchemingSeer.RaffineSchemingSeerEffect", apply)];

/// `ConniveSourceEffect.connive(target or its last known self, attacking creatures)`: its controller draws X, then
/// discards X; a +1/+1 counter for each nonland card discarded, when it's still on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(o)) = cx.item.targets.first().and_then(|t| t.first()).copied() else { return Some(()) };
    let amount =
        game.battlefield.iter().filter(|a| game.has_type(**a, CardType::Creature) && game.obj(**a).attacking.is_some()).count()
            as i32;
    if amount < 1 {
        return Some(());
    }
    let still = game.obj(o).zone == Zone::Battlefield;
    let zcc = game.obj(o).zcc;
    let p = game.obj(o).controller;
    draw(game, p, amount as usize);
    let gone = discard(game, d, p, amount);
    if d.stopped() {
        return None;
    }
    let n = gone.iter().filter(|c| !game.has_type(**c, CardType::Land)).count() as i32;
    if still && n > 0 && game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
        put_counters(game, o, "+1/+1", n, p);
    }
    Some(())
}

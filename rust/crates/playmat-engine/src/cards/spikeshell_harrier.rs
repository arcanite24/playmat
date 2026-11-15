//! Spikeshell Harrier: "When this creature enters, return target creature or Vehicle an opponent controls to its
//! owner's hand. If that opponent's speed is greater than each other player's speed, reduce that opponent's speed by
//! 1. This effect can't reduce their speed below 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpikeshellHarrier.SpikeshellHarrierEffect", apply)];

/// the target to its owner's hand; then its controller's speed down one (not below 1: `decreaseSpeed`) if it's above
/// every other player's
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    let player = game.obj(target).controller;
    game.move_to(target, Zone::Hand);
    let max = (0..game.seats())
        .filter(|p| *p != player && !game.player(*p).lost)
        .map(|p| game.player(p).speed)
        .max()
        .unwrap_or(0);
    let speed = game.player(player).speed;
    if speed > max && speed > 1 {
        game.player_mut(player).speed = speed - 1;
    }
    Some(())
}

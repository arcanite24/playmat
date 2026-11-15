//! Chainer's Torment: "III — Create an X/X black Nightmare Horror creature token, where X is half your life total,
//! rounded up. It deals X damage to you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChainersTorment.ChainersTormentEffect", apply)
    .tokens(&["ChainersTormentNightmareToken"])];

/// X is half your life (at least 0), rounded up; the token, then each token made deals X damage to you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = (game.player(you).life.max(0) + 1) / 2;
    let mut token = cx.tokens[0].clone();
    token.power = x;
    token.toughness = x;
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &token, 1);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        deal_damage(game, t, Target::Player(you), x, false);
    }
    Some(())
}

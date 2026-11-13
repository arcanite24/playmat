//! Peer into the Abyss: "Target player draws cards equal to half the number of cards in their library and loses
//! half their life. Round up each time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PeerIntoTheAbyss.PeerIntoTheAbyssEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let n = game.player(p).library.len().div_ceil(2);
    draw(game, p, n);
    let life = game.player(p).life;
    let lose = if life > 0 { (life + 1) / 2 } else { life / 2 };
    if lose > 0 {
        lose_life(game, p, lose);
    }
    Some(())
}

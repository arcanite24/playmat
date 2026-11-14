//! Stormchaser Chimera: "{2}{U}{R}: Scry 1, then reveal the top card of your library. Stormchaser Chimera gets
//! +X/+0 until end of turn, where X is that card's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StormchaserChimera.StormchaserChimeraEffect", apply)];

/// The top card revealed; the Chimera, while it's still the permanent that activated, gets +X/+0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let src = cx.source_id();
    game.revealed.push(crate::state::Revealed {
        source: src,
        zcc: game.obj(src).zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    let mv = game.obj(card).def.mana_value() as i32;
    if cx.source_on_battlefield(game) {
        boost(game, src, mv, 0, Duration::EndOfTurn);
    }
    Some(())
}

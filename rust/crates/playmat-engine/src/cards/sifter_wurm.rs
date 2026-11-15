//! Sifter Wurm: "When Sifter Wurm enters, scry 3, then reveal the top card of your library. You gain life equal to
//! that card's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SifterWurm.SifterWurmEffect", apply)];

/// Scry 3; then the top card is revealed and you gain its mana value
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    if !scry(game, d, you, 3, &rule) {
        return None;
    }
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    let n = game.obj(card).def.mana_value() as i32;
    gain_life(game, you, n);
    Some(())
}

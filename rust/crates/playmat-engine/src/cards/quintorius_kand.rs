//! Quintorius Kand: "−6: Exile any number of target cards from your graveyard. Add {R} for each card exiled this way.
//! You may play those cards this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("QuintoriusKand.QuintoriusKandEffect", apply)];

/// The legal targets are exiled; {R} for each of them (Java counts the targets, not what moved), and each one in
/// exile is playable this turn (`makeCardPlayable(…, false, EndOfTurn, false)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards = cx.objs(0);
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    game.player_mut(you).mana_pool.colored[Color::R.index()] += cards.len() as u32;
    let turn = game.turn;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).may_play = Some((you, turn));
        }
    }
    Some(())
}

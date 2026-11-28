//! Parker Luck: "At the beginning of your end step, two target players each reveal the top card of their library.
//! They each lose life equal to the mana value of the card revealed by the other player. Then they each put the card
//! they revealed into their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ParkerLuck.ParkerLuckEffect", apply)];

/// each top card revealed (the first player's, then the second's), the life lost crosswise, then each to its hand
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let players: Vec<Seat> = cx
        .legal
        .first()?
        .iter()
        .filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None })
        .collect();
    let (&one, &two) = (players.first()?, players.get(1)?);
    let top = |game: &Game, p: Seat| game.player(p).library.first().copied();
    let (card_one, card_two) = (top(game, one), top(game, two));
    let zcc = game.obj(cx.source_id()).zcc;
    let mut mv = [0, 0];
    for (k, card) in [card_one, card_two].into_iter().enumerate() {
        if let Some(c) = card {
            game.revealed.push(crate::state::Revealed {
                source: cx.source_id(),
                zcc,
                paren: false,
                cards: vec![(c, Zone::Library)],
            });
            mv[k] = game.obj(c).def.mana_value() as i32;
        }
    }
    lose_life(game, one, mv[1]);
    lose_life(game, two, mv[0]);
    for c in [card_one, card_two].into_iter().flatten() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}

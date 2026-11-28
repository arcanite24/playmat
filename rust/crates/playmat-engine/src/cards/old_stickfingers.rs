//! Old Stickfingers: "When you cast this spell, reveal cards from the top of your library until you reveal X creature
//! cards. Put all creature cards revealed this way into your graveyard and the rest on the bottom of your library in
//! a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OldStickfingers.OldStickfingersEffect", apply)];

/// The spell's X (as it's still on the stack); none, nothing. Every card revealed, from the top, until the X-th
/// creature card (or the whole library); the creatures to the graveyard together, the rest to the bottom at random
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let x = game
        .stack
        .iter()
        .rev()
        .find(|i| i.card == src && i.ability.is_none() && i.delayed.is_none())
        .map_or(game.obj(src).cast_x, |i| i.x);
    if x < 1 {
        return Some(());
    }
    let you = cx.controller();
    let (mut revealed, mut creatures, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for c in game.player(you).library.clone() {
        revealed.push(c);
        if game.has_type(c, CardType::Creature) {
            creatures.push(c);
            if creatures.len() as i32 == x {
                break;
            }
        } else {
            others.push(c);
        }
    }
    if !revealed.is_empty() {
        let zcc = game.obj(src).zcc;
        let cards = revealed.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards });
    }
    for c in creatures {
        game.move_to(c, Zone::Graveyard);
    }
    if !others.is_empty() && !put_cards(game, d, you, &others, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}

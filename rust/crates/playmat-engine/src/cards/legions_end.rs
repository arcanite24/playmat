//! Legion's End: "Exile target creature an opponent controls with mana value 2 or less and all other creatures that
//! player controls with the same name as that creature. Then that player reveals their hand and exiles all cards
//! with that name from their hand and graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LegionsEnd.LegionsEndEffect", apply)];

/// the creatures, the hand's and the graveyard's cards of that name, exiled together after the reveal
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(o).controller;
    let name = game.obj(o).name().to_string();
    let mut cards: Vec<ObjId> = if name.is_empty() {
        vec![o]
    } else {
        game.battlefield
            .iter()
            .copied()
            .filter(|b| {
                game.obj(*b).controller == p && game.has_type(*b, CardType::Creature) && game.obj(*b).name() == name
            })
            .collect()
    };
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    if !name.is_empty() {
        cards.extend(hand.iter().copied().filter(|c| game.obj(*c).name() == name));
        cards.extend(game.player(p).graveyard.iter().copied().filter(|c| game.obj(*c).name() == name));
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}

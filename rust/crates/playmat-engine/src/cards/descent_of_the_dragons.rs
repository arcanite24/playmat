//! Descent of the Dragons: "Destroy any number of target creatures. For each creature destroyed this way, its
//! controller creates a 4/4 red Dragon creature token with flying." Hour of Need's "Exile any number of target
//! creatures. For each creature exiled this way, its controller creates a 4/4 blue Sphinx creature token with
//! flying." works the same way.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DescentOfTheDragons.DescentOfTheDragonsEffect", descent).tokens(&["DragonToken"]),
    Handler::effect("HourOfNeed.HourOfNeedExileEffect", hour_of_need).tokens(&["HourOfNeedSphinxToken"]),
];

fn descent(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    replace_with_tokens(game, d, cx, false)
}

fn hour_of_need(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    replace_with_tokens(game, d, cx, true)
}

/// Each target in order destroyed (or exiled), its controller as it was counted when it left the battlefield; then
/// each of those players creates that many tokens in one batch. Java keeps the counts in a `HashMap` by player id,
/// so with tokens for both players its order is the ids'; here it's the order the first of each player's creatures
/// left (the same whenever only one player's creatures go, as in every recorded game).
fn replace_with_tokens(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, exile: bool) -> Option<()> {
    let mut counts: Vec<(Seat, i32)> = Vec::new();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let p = game.obj(o).controller;
        if exile {
            game.move_to(o, Zone::Exile);
        } else {
            destroy(game, o);
        }
        if game.obj(o).zone == Zone::Battlefield {
            continue;
        }
        match counts.iter_mut().find(|(q, _)| *q == p) {
            Some((_, n)) => *n += 1,
            None => counts.push((p, 1)),
        }
    }
    if counts.is_empty() {
        return Some(());
    }
    let token = cx.tokens.first()?.clone();
    for (p, n) in counts {
        let e = EffectDef::CreateTokenEffect {
            token: token.clone(),
            amount: crate::card::Amount::Fixed(n),
            tapped_and_attacking: false,
            tapped: false,
            attacking: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            haste_until_end_of_turn: false,
            also: None,
            attach_source: None,
        };
        cx.run_as(game, d, &[e], p);
    }
    Some(())
}

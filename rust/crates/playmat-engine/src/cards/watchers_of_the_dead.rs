//! Watchers of the Dead: "Exile Watchers of the Dead: Each opponent chooses 2 cards in their graveyard and exiles the
//! rest."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WatchersOfTheDead.WatchersOfTheDeadEffect", apply)];

/// Each opponent in range with two or more cards in their graveyard keeps two (`TargetCardInYourGraveyard(2, 2)`,
/// `Player.choose`) and exiles the rest, one at a time
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        let graveyard = game.player(p).graveyard.clone();
        if game.player(p).lost || graveyard.len() < 2 {
            continue;
        }
        let keep = choose_cards(game, d, p, &graveyard, &graveyard, 2, 2, &rule)?;
        for c in graveyard {
            if !keep.contains(&c) && game.obj(c).zone == Zone::Graveyard {
                game.move_to(c, Zone::Exile);
            }
        }
    }
    Some(())
}

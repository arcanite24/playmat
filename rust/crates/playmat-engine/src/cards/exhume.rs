//! Exhume: "Each player puts a creature card from their graveyard onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Exhume.ExhumeEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut to_enter: Vec<(ObjId, Seat)> = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures: Vec<ObjId> = game
            .player(p)
            .graveyard
            .iter()
            .copied()
            .filter(|c| game.obj(*c).def.is(CardType::Creature))
            .collect();
        if creatures.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?;
        if let Some(&c) = chosen.first() {
            to_enter.push((c, p));
        }
    }
    if to_enter.is_empty() {
        return Some(());
    }
    let cards: Vec<ObjId> = to_enter.iter().map(|(c, _)| *c).collect();
    game.entering_together = cards;
    for (c, p) in to_enter {
        if game.obj(c).zone == Zone::Graveyard {
            enter_battlefield(game, d, c, p, false);
        }
    }
    game.entering_together.clear();
    (!d.stopped()).then_some(())
}

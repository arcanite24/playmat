//! Worlds Within Worlds: "Exile all creatures. Each player may put any number of creature cards from their hand onto
//! the battlefield. Then put all cards exiled this way into their owners' hands. Exile Worlds Within Worlds."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WorldsWithinWorlds.WorldsWithinWorldsEffect", apply)];

/// Each creature exiled in turn (the cards that are now in exile are kept); each player in range from you chooses
/// any number of creature cards in their hand (`TargetCardInHand(0, MAX)`), and they all enter together, each
/// under its owner's control (`moveCards(…, byOwner)`); then the exiled cards go to their owners' hands
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let mut exiled = Vec::new();
    for o in creatures {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile && !game.obj(o).token {
            exiled.push(o);
        }
    }
    let n = game.seats();
    let players: Vec<Seat> = (0..n).map(|k| (you + k) % n).filter(|p| !game.player(*p).lost).collect();
    let rule = item_rule(game, cx.item);
    let mut chosen: Vec<ObjId> = Vec::new();
    for p in players {
        let hand = game.player(p).hand.clone();
        let possible: Vec<ObjId> = hand.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
        chosen.extend(choose_cards(game, d, p, &hand, &possible, 0, usize::MAX, &rule)?);
    }
    let cards: Vec<ObjId> = chosen.into_iter().filter(|c| game.obj(*c).zone == Zone::Hand).collect();
    if !cards.is_empty() {
        // `put_cards`' battlefield move, each card under its owner's control
        game.entering_together = cards.clone();
        let pending_before = game.pending.len();
        for c in &cards {
            let owner = game.obj(*c).owner;
            enter_battlefield(game, d, *c, owner, false);
        }
        game.entering_together.clear();
        if game.pending.len() > pending_before + 1 {
            let mut fired = game.pending.split_off(pending_before);
            let key = |g: &Game, t: &StackItem| {
                let event = t.entering.unwrap_or(t.card);
                let at = cards.iter().position(|c| *c == event).unwrap_or(cards.len());
                (at, g.trigger_key(t))
            };
            fired.sort_by_key(|t| key(game, t));
            game.pending.extend(fired);
        }
        if d.stopped() {
            return None;
        }
    }
    for c in exiled {
        if game.obj(c).zone == Zone::Exile {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

//! Intrude on the Mind: "Reveal the top five cards of your library and separate them into two piles. An opponent
//! chooses one of those piles. Put that pile into your hand and the other into your graveyard. Create a 0/0
//! colorless Thopter artifact creature token with flying, then put a +1/+1 counter on it for each card put into your
//! graveyard this way."

use super::*;
use crate::card::PutCards as P;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("IntrudeOnTheMind.IntrudeOnTheMindEffect", apply).tokens(&["Thopter00ColorlessToken"])];

/// The top five revealed; you choose the first pile (`choose(outcome, cards, TargetCard(0, all))`), the opponent's
/// `choosePile` (yes: the first pile to hand); the first pile moves, then the second; the Thopter gets a +1/+1
/// counter for each of the five now in the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let rule = item_rule(game, cx.item);
    let pile1 = if cards.is_empty() {
        Vec::new()
    } else {
        choose_cards(game, d, you, &cards, &cards, 0, cards.len(), &rule)?
    };
    let pile2: Vec<ObjId> = cards.iter().copied().filter(|c| !pile1.contains(c)).collect();
    let n = game.seats();
    let Some(opponent) = (1..n).map(|k| (you + k) % n).find(|p| !game.player(*p).lost) else { return Some(()) };
    let first_to_hand = choose_use(game, d, opponent, "Choose a pile to put into hand.")?;
    let (z1, z2) = if first_to_hand { (P::Hand, P::Graveyard) } else { (P::Graveyard, P::Hand) };
    if !put_cards(game, d, you, &pile1, z1) || !put_cards(game, d, you, &pile2, z2) {
        return None;
    }
    let before = game.objs.len();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    if d.stopped() {
        return None;
    }
    let count = cards.iter().filter(|c| game.obj(**c).zone == Zone::Graveyard).count() as i32;
    if count > 0 {
        let made: Vec<ObjId> = (before..game.objs.len())
            .map(|i| ObjId(i as u32))
            .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
            .collect();
        for t in made {
            add_counters_by(game, t, "+1/+1", count, Some(you));
        }
    }
    Some(())
}

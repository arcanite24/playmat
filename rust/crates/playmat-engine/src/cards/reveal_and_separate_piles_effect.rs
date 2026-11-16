//! `RevealAndSeparatePilesEffect(amount, separates, chooses, zone)`: "reveal the top five cards of your library. An
//! opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard"
//! (Fact or Fiction, Steam Augury, Sphinx of Uthuun, Epiphany at the Drownyard, Sphinx of Clear Skies; Jace,
//! Architect of Thought's to the bottom of the library in any order)

use super::*;
use crate::card::PutCards as P;

// (the any-order flag of the five-argument constructor: only its default)
pub const HANDLERS: &[Handler] = &[Handler::effect("RevealAndSeparatePilesEffect", apply).args(&[
    "amount",
    "one_of:TargetController.YOU|TargetController.OPPONENT",
    "one_of:TargetController.YOU|TargetController.OPPONENT",
    "one_of:Zone.GRAVEYARD|Zone.LIBRARY",
    "one_of:true",
])];

/// `doPiles`: the top cards revealed; the separating player chooses the first pile (`choose(outcome, cards,
/// TargetCard(0, all))`), the choosing player `choosePile` (yes: the first pile to hand); the first pile moves, then
/// the second, the one not in hand to the graveyard or the bottom of the library in any order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.arg_amount(game, 0)?;
    if n < 1 {
        return Some(());
    }
    let who = |game: &Game, i: usize| -> Option<Seat> {
        if cx.args.get(i).is_some_and(|a| a == "TargetController.YOU") {
            return Some(you);
        }
        let seats = game.seats();
        (1..seats).map(|k| (you + k) % seats).find(|p| !game.player(*p).lost)
    };
    let cards: Vec<ObjId> = game.player(you).library.iter().take(n as usize).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let Some(separates) = who(game, 1) else { return Some(()) };
    let rule = item_rule(game, cx.item);
    let pile1 = if cards.is_empty() {
        Vec::new()
    } else {
        choose_cards(game, d, separates, &cards, &cards, 0, cards.len(), &rule)?
    };
    let pile2: Vec<ObjId> = cards.iter().copied().filter(|c| !pile1.contains(c)).collect();
    let Some(chooses) = who(game, 2) else { return Some(()) };
    let first_to_hand = choose_use(game, d, chooses, "Choose a pile to put into hand.")?;
    let other = if cx.args.get(3).is_some_and(|a| a == "Zone.LIBRARY") { P::BottomAny } else { P::Graveyard };
    let (z1, z2) = if first_to_hand { (P::Hand, other) } else { (other, P::Hand) };
    if !put_cards(game, d, you, &pile1, z1) || !put_cards(game, d, you, &pile2, z2) {
        return None;
    }
    Some(())
}

//! Curator of Destinies: "When this creature enters, look at the top five cards of your library and separate them
//! into a face-down pile and a face-up pile. An opponent chooses one of those piles. Put that pile into your hand and
//! the other into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CuratorOfDestinies.CuratorOfDestiniesEffect", apply)];

/// `choose(outcome, cards, TargetCard(0, any))` for the face-down pile, the rest revealed, then the opponent's
/// `chooseUse` (yes: the face-down pile goes to hand); the face-down pile moves first
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    let rule = item_rule(game, cx.item);
    let down = if cards.is_empty() {
        Vec::new()
    } else {
        choose_cards(game, d, you, &cards, &cards, 0, cards.len(), &rule)?
    };
    let up: Vec<ObjId> = cards.iter().copied().filter(|c| !down.contains(c)).collect();
    if !up.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = up.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let opponent = game.opponent(you);
    let message = format!("Choose a pile to put into {}'s hand.", log::player(you));
    let down_to_hand = choose_use(game, d, opponent, &message)?;
    let (hand, graveyard) = if down_to_hand { (P::Hand, P::Graveyard) } else { (P::Graveyard, P::Hand) };
    if !put_cards(game, d, you, &down, hand) || !put_cards(game, d, you, &up, graveyard) {
        return None;
    }
    Some(())
}

use crate::card::PutCards as P;

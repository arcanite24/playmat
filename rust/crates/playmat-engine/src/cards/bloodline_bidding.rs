//! Bloodline Bidding: "Choose a creature type. Return all creature cards of the chosen type from your graveyard to
//! the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BloodlineBidding.BloodlineBiddingEffect", apply)];

/// `ChoiceCreatureType`, asked of you; then every creature card of that type in your graveyard moves to the
/// battlefield together (`moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Creature) && game.has_subtype(*c, &chosen))
        .collect();
    if !cards.is_empty() {
        put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}

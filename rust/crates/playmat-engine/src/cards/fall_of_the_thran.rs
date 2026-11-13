//! Fall of the Thran, chapters II and III: "Each player returns two land cards from their graveyard to the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FallOfTheThran.FallOfTheThranReturnEffect", apply)];

/// Each player in turn order from you with two or more land cards in their graveyard chooses two
/// (`TargetCardInYourGraveyard(2, 2)`, not targeted); then each player's two enter the battlefield under them, in
/// that order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut back: Vec<(Seat, Vec<ObjId>)> = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let lands: Vec<ObjId> =
            game.player(p).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
        if lands.len() < 2 {
            continue;
        }
        let rule = item_rule(game, cx.item);
        let chosen = choose_cards(game, d, p, &lands, &lands, 2, 2, &rule)?;
        if chosen.len() == 2 {
            back.push((p, chosen));
        }
    }
    for (p, cards) in back {
        let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
        if !cards.is_empty() && !put_cards(game, d, p, &cards, crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}

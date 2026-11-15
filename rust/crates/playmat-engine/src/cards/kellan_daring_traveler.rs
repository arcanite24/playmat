//! Kellan, Daring Traveler: "Whenever Kellan, Daring Traveler attacks, reveal the top card of your library. If it's a
//! creature card with mana value 3 or less, put it into your hand. Otherwise, you may put it into your graveyard.
//! // Journey On {G}: Create X Map tokens, where X is one plus the number of opponents who control an artifact."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("KellanDaringTraveler.KellanDynamicValue", opponents_with_artifacts),
    Handler::effect("KellanDaringTraveler.KellanDaringTravelerEffect", apply),
];

/// the opponents who control an artifact
fn opponents_with_artifacts(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    (0..game.seats())
        .filter(|p| {
            *p != you
                && game.battlefield.iter().any(|o| game.obj(*o).controller == *p && game.has_type(*o, CardType::Artifact))
        })
        .count() as i32
}

/// The top card revealed; a creature card with mana value 3 or less goes to your hand, anything else to your
/// graveyard if you like
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if game.obj(card).is(CardType::Creature) && game.obj(card).def.mana_value() <= 3 {
        game.move_to(card, Zone::Hand);
    } else {
        let message = format!("Put {} in your graveyard?", game.obj(card).name());
        if choose_use(game, d, you, &message)? {
            game.move_to(card, Zone::Graveyard);
        }
    }
    Some(())
}

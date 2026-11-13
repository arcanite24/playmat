//! Kinscaer Sentry: "Whenever this creature attacks, you may put a creature card with mana value X or less from your
//! hand onto the battlefield tapped and attacking, where X is the number of attacking creatures you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("KinscaerSentry.KinscaerSentryPredicate", at_most_your_attackers)];

/// Its mana value is at most the number of attacking creatures you control
fn at_most_your_attackers(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let attackers = game
        .battlefield
        .iter()
        .filter(|o| {
            let p = game.obj(**o);
            p.controller == you && p.attacking.is_some() && game.has_type(**o, CardType::Creature)
        })
        .count() as i32;
    game.obj(id).def.mana_value() as i32 <= attackers
}

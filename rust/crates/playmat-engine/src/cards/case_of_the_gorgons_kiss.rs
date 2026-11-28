//! Case of the Gorgon's Kiss: "When this Case enters, destroy up to one target creature that was dealt damage this
//! turn. To solve — Three or more creature cards were put into graveyards from anywhere this turn. Solved — This
//! Case is a 4/4 Gorgon creature with deathtouch and lifelink in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("CaseOfTheGorgonsKiss.CaseOfTheGorgonsKissCondition", three_creature_cards)];

/// `CardsPutIntoGraveyardWatcher.getCardsPutIntoGraveyardFromAnywhere`: the cards put into a graveyard this turn
/// that are still the cards put there (`MageObjectReference.getCard`), each once; three or more creature cards
fn three_creature_cards(game: &Game, _: Seat, _: ObjId) -> bool {
    let mut seen: Vec<ObjId> = Vec::new();
    for (c, zcc, _) in &game.watch.to_graveyard {
        let o = game.obj(*c);
        if o.zcc == *zcc && !o.token && o.def.is(CardType::Creature) && !seen.contains(c) {
            seen.push(*c);
        }
    }
    seen.len() >= 3
}

//! `CreaturePutInYourGraveyardCondition`: "a creature card was put into your graveyard from anywhere this turn"
//! (`CreaturePutIntoGraveyardWatcher`; Macabre Reconstruction's cost reduction)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CreaturePutInYourGraveyardCondition", creature_card)];

/// a creature card you own went to a graveyard this turn (and is still the card that did)
fn creature_card(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.to_graveyard.iter().any(|(c, _, _)| game.obj(*c).owner == you && game.obj(*c).def.is(CardType::Creature))
}

//! Part in Friendship: "Whenever a nontoken creature you control dies, reveal cards from the top of your library until
//! you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it
//! onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random
//! order. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PartInFriendship.PartInFriendshipEffect", apply)];

/// The revealed cards up to the first creature card; it goes onto the battlefield or into your hand, the rest to the
/// bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    let mut creature = None;
    for c in game.player(you).library.clone() {
        cards.push(c);
        if game.has_type(c, CardType::Creature) {
            creature = Some(c);
            break;
        }
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    if let Some(c) = creature {
        let lands = game
            .battlefield
            .iter()
            .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
            .count();
        if game.obj(c).def.mana_value() as usize <= lands {
            if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
                return None;
            }
        } else {
            game.move_to(c, Zone::Hand);
        }
        cards.retain(|x| *x != c);
    }
    if !put_on_library(game, d, you, &cards, false, false) {
        return None;
    }
    Some(())
}

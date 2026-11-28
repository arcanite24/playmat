//! Inspiration from Beyond: "Mill three cards, then return an instant or sorcery card from your graveyard to your
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InspirationFromBeyond.InspirationFromBeyondEffect", apply)];

/// One instant or sorcery card in your graveyard is taken as it is; more, you choose one (`TargetCardInGraveyard`,
/// not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Instant) || game.has_type(*c, CardType::Sorcery))
        .collect();
    let card = match cards.len() {
        0 => return Some(()),
        1 => cards[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    game.move_to(card, Zone::Hand);
    Some(())
}

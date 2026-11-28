//! Breach the Multiverse: "Each player mills ten cards. For each player, choose a creature or planeswalker card in
//! that player's graveyard. Put those cards onto the battlefield under your control. Then each creature you control
//! becomes a Phyrexian in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BreachTheMultiverse.BreachTheMultiverseEffect", apply)];

/// For each player in range from you, one creature or planeswalker card of their graveyard
/// (`TargetCardInGraveyard`, not targeted; a graveyard without one is skipped), then all of them onto the
/// battlefield under your control together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let graveyard = game.player(p).graveyard.clone();
        let possible: Vec<ObjId> = graveyard
            .iter()
            .copied()
            .filter(|c| game.has_type(*c, CardType::Creature) || game.has_type(*c, CardType::Planeswalker))
            .collect();
        if possible.is_empty() {
            continue;
        }
        cards.extend(choose_cards(game, d, you, &graveyard, &possible, 1, 1, &rule)?);
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

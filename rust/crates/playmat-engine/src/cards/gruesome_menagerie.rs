//! Gruesome Menagerie: "Choose a creature card with mana value 1 in your graveyard, then do the same for creature
//! cards with mana value 2 and 3. Return those cards to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GruesomeMenagerie.GruesomeMenagerieEffect", apply)];

/// Three `player.choose` of one card (not targeted; none to choose, nothing asked), then one move
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for mv in 1..=3 {
        let possible: Vec<ObjId> = game
            .player(you)
            .graveyard
            .iter()
            .copied()
            .filter(|c| game.has_type(*c, CardType::Creature) && game.obj(*c).def.mana_value() == mv)
            .collect();
        if possible.is_empty() {
            continue;
        }
        let rule = item_rule(game, cx.item);
        if let Some(&c) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() {
            cards.push(c);
        }
    }
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

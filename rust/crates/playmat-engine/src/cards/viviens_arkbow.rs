//! Vivien's Arkbow: "{X}, {T}, Discard a card: Look at the top X cards of your library. You may put a creature card
//! with mana value X or less from among them onto the battlefield. Put the rest on the bottom of your library in a
//! random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ViviensArkbow.ViviensArkbowEffect", apply)];

/// `lookAtCards` (titled by the source), `choose(cards, TargetCardInLibrary(0, 1, filter))`, then the rest to the
/// bottom at random
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    // (no cards, no choice: `HumanPlayer.choose` of an empty set)
    if cards.is_empty() {
        return Some(());
    }
    {
        let zcc = game.obj(cx.source_id()).zcc;
        let look = crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: cards.clone() };
        game.looked_at.push(look);
    }
    let fitting: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Creature) && game.obj(*c).def.mana_value() as i32 <= x)
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &fitting, 0, 1, &rule)?;
    if let Some(&card) = chosen.first() {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            cards.retain(|c| *c != card);
        }
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}

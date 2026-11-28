//! Bramble Familiar // Fetch Quest: "Mill seven cards, then put a creature, enchantment, or land card from among
//! cards milled this way onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrambleFamiliar.FetchQuestEffect", apply)];

/// You mill seven; with a creature, enchantment or land card among them, you choose one of them (`choose` with a
/// required `TargetCardInYourGraveyard(1, 1, filter)`, not targeted) and it's put onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let milled: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, milled.len() as i32);
    for c in &milled {
        game.move_to(*c, Zone::Graveyard);
    }
    let milled: Vec<ObjId> = milled.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let possible: Vec<ObjId> = milled
        .iter()
        .copied()
        .filter(|c| [CardType::Creature, CardType::Enchantment, CardType::Land].iter().any(|t| game.has_type(*c, *t)))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &milled, &possible, 1, 1, &rule)?;
    if !chosen.is_empty() {
        put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

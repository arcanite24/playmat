//! Sudden Reclamation: "Put the top four cards of your library into your graveyard, then return a creature card and a
//! land card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SuddenReclamation.SuddenReclamationEffect", apply)];

/// A creature card, then a land card (each not targeted, when there's one), both returned together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut to_hand = Vec::new();
    for t in [CardType::Creature, CardType::Land] {
        let possible: Vec<ObjId> =
            game.player(you).graveyard.iter().copied().filter(|c| game.has_type(*c, t) && !to_hand.contains(c)).collect();
        if possible.is_empty() {
            continue;
        }
        if let Some(&c) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() {
            to_hand.push(c);
        }
    }
    for c in to_hand {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}

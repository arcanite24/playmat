//! Grim Captain's Call: "Return a Pirate card from your graveyard to your hand, then do the same for Vampire,
//! Dinosaur, and Merfolk."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GrimCaptainsCall.GrimCaptainsCallEffect", apply)];

/// For each type in turn: a creature card of it from your graveyard (`chooseTarget`, when there's one) to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    for s in ["Pirate", "Vampire", "Dinosaur", "Merfolk"] {
        let graveyard = game.player(you).graveyard.clone();
        let possible: Vec<ObjId> = graveyard
            .iter()
            .copied()
            .filter(|c| game.obj(*c).is(CardType::Creature) && game.has_subtype(*c, s))
            .collect();
        if possible.is_empty() {
            continue;
        }
        if let Some(&c) = choose_cards(game, d, you, &graveyard, &possible, 1, 1, &rule)?.first() {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

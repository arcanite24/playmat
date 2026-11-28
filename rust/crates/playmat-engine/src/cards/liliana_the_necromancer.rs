//! Liliana, the Necromancer: "−7: Destroy up to two target creatures. Put up to two creature cards from graveyards
//! onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianaTheNecromancer.LilianaTheNecromancerEffect", apply)];

/// `player.choose` of a `TargetCardInGraveyard(0, 2)` (not targeted) over every graveyard (asked
/// with none too: zero is enough), then one move
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = (0..game.seats())
        .flat_map(|p| game.player(p).graveyard.clone())
        .filter(|c| game.has_type(*c, CardType::Creature))
        .collect();
    let chosen = choose_cards_rounds(game, d, you, &cards, 0, 2)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

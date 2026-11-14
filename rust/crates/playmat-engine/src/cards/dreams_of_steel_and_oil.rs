//! Dreams of Steel and Oil: "Target opponent reveals their hand. You choose an artifact or creature card from it, then
//! choose an artifact or creature card from their graveyard. Exile the chosen cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DreamsOfSteelAndOil.DreamsOfSteelAndOilEffect", apply)];

/// The hand is revealed; you choose from it, then from the graveyard (each not targeted, when there's one); both
/// are exiled together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let you = cx.controller();
    let hand = game.player(p).hand.clone();
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    let fits = |game: &Game, c: ObjId| game.has_type(c, CardType::Artifact) || game.has_type(c, CardType::Creature);
    let rule = item_rule(game, cx.item);
    let mut to_exile = Vec::new();
    let possible: Vec<ObjId> = hand.iter().copied().filter(|c| fits(game, *c)).collect();
    if !possible.is_empty() {
        to_exile.extend(choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?);
    }
    let graveyard = game.player(p).graveyard.clone();
    let possible: Vec<ObjId> = graveyard.iter().copied().filter(|c| fits(game, *c)).collect();
    if !possible.is_empty() {
        to_exile.extend(choose_cards(game, d, you, &graveyard, &possible, 1, 1, &rule)?);
    }
    for c in to_exile {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}

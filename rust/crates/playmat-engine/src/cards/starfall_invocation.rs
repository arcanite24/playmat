//! Starfall Invocation: "Gift a card. Destroy all creatures. If the gift was promised, return a creature card put
//! into your graveyard this way to the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StarfallInvocation.StarfallInvocationEffect", apply)];

/// each creature destroyed in turn; with the gift promised, one of your cards that went to your graveyard (a lone
/// one is taken: `TargetCardInYourGraveyard` is required), onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let mut yours = Vec::new();
    for o in creatures {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        destroy(game, o);
        if game.obj(o).zone == Zone::Graveyard && game.obj(o).owner == you {
            yours.push(o);
        }
    }
    if !cx.item.gift_promised || yours.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> = yours
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Graveyard && game.obj(*c).owner == you)
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &yours, &possible, 1, 1, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

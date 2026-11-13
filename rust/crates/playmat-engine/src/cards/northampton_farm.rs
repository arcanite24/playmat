//! Northampton Farm: "{2}, {T}, Sacrifice this land: Return a creature card exiled with this land to the battlefield
//! under your control. Return each other card exiled with this land to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NorthamptonFarm.NorthamptonFarmEffect", apply)];

/// the cards its first ability exiled (by the land as it was: the sacrifice moved it), still in exile; one creature
/// card chosen when there are two or more (`TargetCardInExile`, required)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (src, zcc) = cx.source;
    let mut cards: Vec<ObjId> = Vec::new();
    for (s, z, c) in game.exiled_for_source.iter() {
        if *s == src && (*z == zcc || *z + 1 == zcc) && game.obj(*c).zone == Zone::Exile && !cards.contains(c) {
            cards.push(*c);
        }
    }
    let creatures: Vec<ObjId> = cards.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    let card = match creatures.len() {
        0 => None,
        1 => Some(creatures[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &creatures, &creatures, 1, 1, &rule)?.first().copied()
        }
    };
    if let Some(c) = card {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
        cards.retain(|x| *x != c);
    }
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

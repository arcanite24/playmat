//! Ojer Kaslem, Deepest Growth: "Whenever Ojer Kaslem deals combat damage to a player, reveal that many cards from
//! the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put
//! the rest on the bottom in a random order. When Ojer Kaslem dies, return it to the battlefield tapped and
//! transformed under its owner's control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("OjerKaslemDeepestGrowth.OjerKaslemDeepestGrowthEffect", reveal),
    Handler::effect("OjerKaslemDeepestGrowth.OjerKaslemDeepestGrowthTransformEffect", transform),
];

/// X is the damage dealt (`getValue("damage")`, the trigger's X): the top X revealed, In the Presence of Ages'
/// creature-and/or-land pick (`TargetCardAndOrCardInLibrary`), those onto the battlefield together, the rest to the
/// bottom in a random order
fn reveal(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0) as usize;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(x).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: cards.iter().map(|c| (*c, Zone::Library)).collect(),
    });
    let chosen = super::in_the_presence_of_ages::choose_creature_and_or_land(game, d, you, &cards)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    let rest: Vec<ObjId> =
        cards.into_iter().filter(|c| !chosen.contains(c) && game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}

/// Ojer Taq's return: tapped, its back face up
fn transform(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::ojer_taq_deepest_foundation::return_transformed(game, d, cx, true)
}

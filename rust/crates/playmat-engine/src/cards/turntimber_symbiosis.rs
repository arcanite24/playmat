//! Turntimber Symbiosis: "Look at the top seven cards of your library. You may put a creature card from among them
//! onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on
//! it. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TurntimberSymbiosis.TurntimberSymbiosisEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURE"])];

/// `Player.choose` of up to one creature card among the seven (asked whenever there are cards, as a pile's choice);
/// the counters are `setEnterWithCounters` by the card's id, which a double-faced card's face doesn't find (as
/// `ReturnFromGraveyardToBattlefieldTargetEffect` here)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> =
        top.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &possible, 0, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        let counters = game.obj(c).def.mana_value() <= 3;
        let faced = game.obj(c).def.transform_back.is_some() || game.obj(c).def.back.is_some();
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
        if counters && !faced && game.obj(c).zone == Zone::Battlefield {
            game.obj_mut(c).add_counters("+1/+1", 3);
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| !chosen.contains(c) && game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}

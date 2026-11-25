//! Keldon Flamesage: "Whenever Keldon Flamesage attacks, look at the top X cards of your library, where X is Keldon
//! Flamesage's power. You may exile an instant or sorcery card with mana value X or less from among them. Put the
//! rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KeldonFlamesage.KeldonFlamesageEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_INSTANT_OR_SORCERY"])];

/// X is the source's power (its last known one once it's gone); nothing below 1. `player.choose` of up to one
/// instant or sorcery card of mana value X or less among the top X, its exile, the rest to the bottom in a random
/// order, then `castSpellWithAttributesForFree` of the exiled card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let power = if cx.source_on_battlefield(game) {
        game.power_toughness(cx.source_id()).0
    } else {
        game.obj(cx.source_id()).power_lki?
    };
    if power < 1 {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(power as usize).copied().collect();
    let possible: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| {
            game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)
                && (game.obj(*c).def.mana_value() as i32) < power + 1
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = if top.is_empty() { Vec::new() } else { choose_cards(game, d, you, &top, &possible, 0, 1, &rule)? };
    let card = chosen.first().copied();
    if let Some(c) = card {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Exiled) {
            return None;
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !rest.is_empty() && !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    if let Some(c) = card.filter(|c| game.obj(*c).zone == Zone::Exile) {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, c));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, c);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}

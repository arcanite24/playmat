//! Vision Quest: "Search your library and/or graveyard for an artifact creature card with mana value X or less and
//! put it onto the battlefield with X additional +1/+1 counters on it. If X is 4 or greater, it gains haste until
//! end of turn. If you search your library this way, shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VisionQuest.VisionQuestEffect", apply)];

/// Asked first whether to search the library (up to one); nothing found there, whether to look in the graveyard
/// (`player.choose`, up to one). The card enters under you with X +1/+1 counters, and with X 4 or more haste until
/// end of turn; the library, if searched, is shuffled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "type", "t": "ARTIFACT"}, {"p": "type", "t": "CREATURE"}, {"p": "mv", "cmp": "OR_LESS", "n": x}
    ]))
    .ok()?;
    let mut card = None;
    let mut searched = false;
    if choose_use(game, d, you, "Search your library for an artifact creature card with mana value X or less?")? {
        card = search_library(game, d, you, &filter, 0, 1, cx.source_id())?.first().copied();
        searched = true;
    }
    if card.is_none()
        && choose_use(game, d, you, "Search your graveyard for an artifact creature card with mana value X or less?")?
    {
        let src = cx.source_id();
        let possible: Vec<ObjId> = game
            .player(you)
            .graveyard
            .iter()
            .copied()
            .filter(|c| game.filter_matches(&filter, you, Some(src), *c))
            .collect();
        card = super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)?.first().copied();
    }
    if let Some(card) = card {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            if x > 0 {
                put_counters(game, card, "+1/+1", x, you);
            }
            if x >= 4 {
                grant(game, card, &AbilityDef::HasteAbility);
            }
        }
    }
    if searched {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    Some(())
}

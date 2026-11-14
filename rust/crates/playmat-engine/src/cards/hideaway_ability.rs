//! Hideaway N (`HideawayAbility`): "When this enters, look at the top N cards of your library, exile one face down,
//! then put the rest on the bottom of your library in a random order." (`HideawayExileEffect`), and the
//! `HideawayPlayEffect` that plays what it exiled without paying its mana cost (Fight Rigging, Clive's Hideaway,
//! Rabble Rousing). The engine keeps no face-down exile: the card is only hidden from the opponent in Java.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("HideawayExileEffect", exile_one),
    Handler::effect("HideawayPlayEffect", play_them).args(&["one_of:false"]),
];

/// One of the top N chosen (required) and exiled in the source's exile zone (`CardUtil.getExileZoneId`); the
/// others still in the library to the bottom, shuffled
fn exile_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.arg_int(0)?.max(0) as usize;
    let top: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    if let Some(&card) = choose_cards(game, d, you, &top, &top, 1, 1, "card to exile face down")?.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            let zone = (cx.source_id(), game.obj(cx.source_id()).zcc);
            game.obj_mut(card).exiled_with = Some(zone);
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false);
    Some(())
}

/// The cards in the source's exile zone: `choose(cards, TargetCard(0, all))`, then each in the order chosen played
/// without paying its mana cost; a land only on your turn with a land play left (any step)
fn play_them(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let zone = (cx.source_id(), game.obj(cx.source_id()).zcc);
    let cards: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Exile && game.obj(*c).exiled_with == Some(zone))
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &cards, &cards, 0, cards.len(), "cards to play (in order they're chosen)")?;
    for card in chosen {
        if game.obj(card).zone != Zone::Exile {
            continue;
        }
        if game.obj(card).is(CardType::Land) {
            // only the turn's active player with a land play left: the timing is ignored (`playCard(…, true, …)`)
            if game.active != Some(you) || game.player(you).lands_played >= lands_per_turn(game, you) {
                continue;
            }
            enter_battlefield(game, d, card, you, false);
            game.player_mut(you).lands_played += 1;
            note(game, EventKind::Land, Some(you), Some(card), None, 0);
        } else {
            cast_free(game, d, you, card);
        }
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

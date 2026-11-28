//! Etali, Primal Storm: "Whenever Etali attacks, exile the top card of each player's library, then you may cast any
//! number of spells from among those cards without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EtaliPrimalStorm.EtaliPrimalStormEffect", apply)];

/// The top card of each player's library, from you in turn order, exiled together; then
/// `CardUtil.castMultipleWithAttributeForFree` with a nonland filter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        if let Some(c) = game.player(p).library.first().copied() {
            cards.push(c);
        }
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    cast_multiple_for_free(game, d, cx, you, cards)
}

/// `CardUtil.castMultipleWithAttributeForFree(player, source, game, cards, nonland filter)`: as many casts as there
/// are cards, each `castSpellWithAttributesForFree`; a cast not made asks whether to go on
pub(super) fn cast_multiple_for_free(
    game: &mut Game,
    d: &mut dyn Decider,
    cx: &EffectCx,
    you: Seat,
    mut cards: Vec<ObjId>,
) -> Option<()> {
    let max = cards.len();
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    // `getCastableComponents`: nonland parts, wherever the card is now (targets aren't looked at)
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect()
    };
    if castable(game, &cards).is_empty() {
        return Some(());
    }
    loop {
        let options = castable(game, &cards);
        // `castSpellWithAttributes`: one card is taken as it is, more are chosen among (up to one)
        let card = match options.len() {
            0 => None,
            1 => Some(options[0]),
            _ => {
                let rule = item_rule(game, cx.item);
                choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first().copied()
            }
        };
        let mut cast = false;
        if let Some(card) = card {
            let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
            if choose_use(game, d, you, &message)? {
                cast = cast_free(game, d, you, card);
                if d.stopped() {
                    return None;
                }
            }
        }
        cards.retain(|c| game.obj(*c).zone != Zone::Stack);
        if cards.is_empty() || castable(game, &cards).is_empty() {
            break;
        }
        if cast {
            cast_count += 1;
            if cast_count >= max {
                break;
            }
        } else {
            let message = format!("You can cast {} more spells for free. Continue casting?", max - cast_count);
            if !choose_use(game, d, you, &message)? {
                break;
            }
        }
    }
    Some(())
}

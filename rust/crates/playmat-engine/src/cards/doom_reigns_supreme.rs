//! Doom Reigns Supreme: "Whenever a Villain you control enters, each opponent loses 1 life and you gain 1 life. Put a
//! plan counter on this enchantment. When the fifth plan counter is put on this enchantment, sacrifice it. When you
//! do, target opponent exiles the top five cards of their library. You may cast up to two spells from among the
//! exiled cards without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DoomReignsSupreme.DoomReignsSupremeExileEffect", apply)];

/// The opponent's top five exiled; then `castMultipleWithAttributeForFree(…, FILTER_CARD_NON_LAND, 2)` over those
/// still in exile
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(5).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
    }
    let mut cards: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect()
    };
    let max = cards.len().min(2);
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    if castable(game, &cards).is_empty() {
        return Some(());
    }
    loop {
        let options = castable(game, &cards);
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

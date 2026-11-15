//! Villainous Wealth: "Target opponent exiles the top X cards of their library. You may cast any number of spells
//! with mana value X or less from among them without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VillainousWealth.VillainousWealthEffect", apply)];

/// X 1 or more: the top X cards exiled together, then `CardUtil.castMultipleWithAttributeForFree` with a "mana value
/// X or less" filter (as many casts as cards exiled; Etali, Primal Storm's loop with that filter)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let Some(opp) = cx.player(0) else { return Some(()) };
    if x < 1 {
        return Some(());
    }
    let cards: Vec<ObjId> = game.player(opp).library.iter().take(x as usize).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let mut cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let max = cards.len();
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards
            .iter()
            .copied()
            .filter(|c| !game.obj(*c).is(CardType::Land) && game.obj(*c).def.mana_value() as i32 <= x)
            .collect()
    };
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

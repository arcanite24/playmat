//! Invoke Calamity: "You may cast up to two instant and/or sorcery spells with total mana value 6 or less from your
//! graveyard and/or hand without paying their mana costs. If those spells would be put into your graveyard, exile
//! them instead. Exile Invoke Calamity."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InvokeCalamity.InvokeCalamityEffect", apply)];

/// `CardUtil.castMultipleWithAttributeForFree` over your hand and graveyard, up to two instants and sorceries whose
/// mana values with those already cast come to 6 or less (`InvokeCalamityTracker`); each one cast is exiled instead
/// of put into your graveyard as it leaves the stack (`InvokeCalamityReplacementEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).hand.clone();
    cards.extend(game.player(you).graveyard.iter().copied());
    let max = cards.len().min(2);
    let mut total = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    let castable = |game: &Game, cards: &[ObjId], total: i32| -> Vec<ObjId> {
        cards
            .iter()
            .copied()
            .filter(|c| {
                let o = game.obj(*c);
                !o.is(CardType::Land)
                    && (o.is(CardType::Instant) || o.is(CardType::Sorcery))
                    && o.def.mana_value() as i32 + total <= 6
            })
            .collect()
    };
    if castable(game, &cards, total).is_empty() {
        return Some(());
    }
    let mut cast_count = 0;
    loop {
        let options = castable(game, &cards, total);
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
                if cast {
                    total += game.obj(card).def.mana_value() as i32;
                    game.obj_mut(card).flashback = true;
                }
            }
        }
        cards.retain(|c| game.obj(*c).zone != Zone::Stack);
        if cards.is_empty() || castable(game, &cards, total).is_empty() {
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

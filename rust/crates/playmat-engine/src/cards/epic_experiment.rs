//! Epic Experiment: "Exile the top X cards of your library. For each instant and sorcery card with mana value X or
//! less among them, you may cast that card without paying its mana cost. Then put all cards exiled this way that
//! weren't cast into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EpicExperiment.EpicExperimentEffect", apply)];

/// the top X cards exiled together, then `CardUtil.castMultipleWithAttributeForFree` with an "instant or sorcery
/// card with mana value X or less" filter (Villainous Wealth's loop), then the ones still in exile into your
/// graveyard, in the order they were exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let cards: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let exiled: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let mut cards = exiled.clone();
    let max = cards.len();
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards
            .iter()
            .copied()
            .filter(|c| {
                let o = game.obj(*c);
                (o.is(CardType::Instant) || o.is(CardType::Sorcery)) && o.def.mana_value() as i32 <= x
            })
            .collect()
    };
    if !castable(game, &cards).is_empty() {
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
    }
    for c in exiled {
        if game.obj(c).zone == Zone::Exile {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}

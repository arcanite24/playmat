//! Back for Seconds: "You may bargain this spell as you cast it. Return up to two target creature cards from your
//! graveyard to your hand. If this spell was bargained, you may put one of those cards with mana value 4 or less onto
//! the battlefield instead of putting it into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BackForSeconds.BackForSecondsEffect", apply)];

/// (bargained) the targets still in the graveyard: up to one with mana value 4 or less chosen onto the battlefield,
/// then the rest to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> =
        cx.legal.iter().flatten().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect();
    cards.retain(|c| game.obj(*c).zone == Zone::Graveyard);
    let small: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).def.mana_value() <= 4).collect();
    if !small.is_empty() {
        let rule = item_rule(game, cx.item);
        let chosen = choose_cards(game, d, you, &cards, &small, 0, 1, &rule)?;
        if let Some(&c) = chosen.first() {
            if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
                return None;
            }
            cards.retain(|x| *x != c);
        }
    }
    for c in cards {
        if game.obj(c).zone == Zone::Graveyard {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

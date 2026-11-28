//! Kiora, Sovereign of the Deep: "Whenever you cast a Kraken, Leviathan, Octopus, or Serpent spell from your hand,
//! look at the top X cards of your library, where X is that spell's mana value. You may cast a spell with mana value
//! less than X from among them without paying its mana cost. Put the rest on the bottom of your library in a random
//! order."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KioraSovereignOfTheDeep.KioraSovereignOfTheDeepEffect", apply)];

/// X is the cast spell's mana value (`getValue("spellCast")`, X included); below 1, nothing. Then Aetherworks
/// Marvel's `castSpellWithAttributesForFree` over the top X with a "mana value less than X" filter, and the cards
/// still in the library go to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let Some(x) = game.watch.spells.iter().rev().find(|w| w.card == spell).map(|w| w.mv) else { return Some(()) };
    if x < 1 {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    let castable: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| !game.obj(*c).is(CardType::Land) && (game.obj(*c).def.mana_value() as i32) < x)
        .collect();
    let card = match castable.len() {
        0 => None,
        1 => Some(castable[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &castable, &castable, 0, 1, &rule)?.first().copied()
        }
    };
    if let Some(card) = card {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}

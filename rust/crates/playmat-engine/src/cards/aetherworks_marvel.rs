//! Aetherworks Marvel: "{T}, Pay {E}{E}{E}{E}{E}{E}: Look at the top six cards of your library. You may cast a spell
//! from among them without paying its mana cost. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AetherworksMarvel.AetherworksMarvelEffect", apply)];

/// `CardUtil.castSpellWithAttributesForFree` over the top six: the nonland cards (one is taken as it is, more are
/// chosen among, up to one), "Cast spell without paying its mana cost (…)?", then the cards still in the library go
/// to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let castable: Vec<ObjId> = top.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
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

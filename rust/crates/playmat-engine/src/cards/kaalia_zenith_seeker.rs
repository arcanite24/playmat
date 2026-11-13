//! Kaalia, Zenith Seeker: "When Kaalia enters, look at the top six cards of your library. You may reveal an Angel
//! card, a Demon card, and/or a Dragon card from among them and put them into your hand. Put the rest on the bottom
//! of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KaaliaZenithSeeker.KaaliaZenithSeekerEffect", apply)];

/// one `choose(cards, TargetCardInLibrary(0, 1, a <type> card))` per type, Angel, Demon, Dragon, from all six each time
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let mut to_hand: Vec<ObjId> = Vec::new();
    if !cards.is_empty() {
        let rule = item_rule(game, cx.item);
        for t in ["Angel", "Demon", "Dragon"] {
            let possible: Vec<ObjId> = cards.iter().copied().filter(|c| game.has_subtype(*c, t)).collect();
            for c in choose_cards(game, d, you, &cards, &possible, 0, 1, &rule)? {
                if !to_hand.contains(&c) {
                    to_hand.push(c);
                }
            }
        }
    }
    if !to_hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = to_hand.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    for c in &to_hand {
        game.move_to(*c, Zone::Hand);
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| !to_hand.contains(c)).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}

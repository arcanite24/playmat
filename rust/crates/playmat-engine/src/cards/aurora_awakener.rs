//! Aurora Awakener: "Trample. Vivid — When this creature enters, reveal cards from the top of your library until you
//! reveal X permanent cards, where X is the number of colors among permanents you control. Put any number of those
//! permanent cards onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a
//! random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AuroraAwakener.AuroraAwakenerEffect", apply)];

/// X from `ColorsAmongControlledPermanentsCount.ALL_PERMANENTS` (nothing below 1); the cards revealed, any number of
/// the permanent cards among them chosen and put onto the battlefield together, the rest to the bottom at random
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let count = colors_among(game, you, cx.source_id(), &crate::card::Filter::default()).len();
    if count < 1 {
        return Some(());
    }
    let mut cards = Vec::new();
    for c in game.player(you).library.clone() {
        cards.push(c);
        if cards.iter().filter(|x| game.obj(**x).def.is_permanent_card()).count() >= count {
            break;
        }
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let possible: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).def.is_permanent_card()).collect();
    let rule = item_rule(game, cx.item);
    let chosen =
        if possible.is_empty() { Vec::new() } else { choose_cards(game, d, you, &cards, &possible, 0, usize::MAX, &rule)? };
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}

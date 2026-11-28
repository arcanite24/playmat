//! Expand the Sphere: "Look at the top six cards of your library. You may put up to two land cards from among them
//! onto the battlefield tapped and the rest on the bottom of your library in a random order. If you put fewer than
//! two lands onto the battlefield this way, proliferate a number of times equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExpandTheSphere.ExpandTheSphereEffect", apply)];

/// `LookLibraryControllerEffect(6)`: the top six looked at; up to two lands among them (`TargetCard(0, 2)`,
/// `chooseTarget`) onto the battlefield tapped, the rest on the bottom at random; a proliferate for each land short
/// of two
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: cards.clone() });
    let lands: Vec<ObjId> = cards.iter().copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let picked = if lands.is_empty() { Vec::new() } else { choose_cards(game, d, you, &cards, &lands, 0, 2, &rule)? };
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !picked.contains(c)).collect();
    if !picked.is_empty() && !put_cards(game, d, you, &picked, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    let rest: Vec<ObjId> = rest.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !rest.is_empty() && !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    for _ in picked.len()..2 {
        if !proliferate(game, d, you) {
            return None;
        }
    }
    Some(())
}

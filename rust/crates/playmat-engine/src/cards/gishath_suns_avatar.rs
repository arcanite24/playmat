//! Gishath, Sun's Avatar: "Whenever Gishath deals combat damage to a player, reveal that many cards from the top of
//! your library. Put any number of Dinosaur creature cards from among them onto the battlefield and the rest on the
//! bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GishathSunsAvatar.GishathSunsAvatarEffect", apply)];

/// The damage (the trigger's `x`) many cards revealed; `choose(cards, TargetCard(0, any, Dinosaur creature))`
/// (asked even with none of them to pick), those onto the battlefield, the rest on the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    reveal_put_rest(game, d, cx, you, cx.item.x.max(0) as usize, crate::card::PutCards::BottomRandom, |g, c| {
        g.has_type(c, CardType::Creature) && g.has_subtype(c, "Dinosaur")
    })
}

/// Reveal the top `n` cards of `you`'s library, put any number of those `pick` allows onto the battlefield, the rest
/// to `rest`
pub(super) fn reveal_put_rest(
    game: &mut Game,
    d: &mut dyn Decider,
    cx: &EffectCx,
    you: Seat,
    n: usize,
    rest: crate::card::PutCards,
    pick: impl Fn(&Game, ObjId) -> bool,
) -> Option<()> {
    let cards: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let possible: Vec<ObjId> = cards.iter().copied().filter(|c| pick(game, *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &possible, 0, cards.len(), &rule)?;
    let others: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) || !put_cards(game, d, you, &others, rest) {
        return None;
    }
    Some(())
}

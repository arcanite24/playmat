//! Bucolic Ranch: "{3}, {T}: Look at the top card of your library. If it's a Mount card, you may reveal it and put
//! it into your hand. If you don't put it into your hand, you may put it on the bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BucolicRanch.BucolicRanchEffect", apply)];

/// The top card looked at; a Mount card, "Put <name> into your hand?" (revealed, then to the hand); still in the
/// library, "Put <name> on the bottom of your library?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(src).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: src, zcc, bare: false, cards: vec![card] });
    let name = game.obj(card).name().to_string();
    if game.has_subtype(card, "Mount") && choose_use(game, d, you, &format!("Put {name} into your hand?"))? {
        game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: vec![(card, Zone::Library)] });
        game.move_to(card, Zone::Hand);
    }
    if game.obj(card).zone == Zone::Library
        && choose_use(game, d, you, &format!("Put {name} on the bottom of your library?"))?
    {
        let lib = &mut game.player_mut(you).library;
        lib.retain(|x| *x != card);
        lib.push(card);
    }
    Some(())
}

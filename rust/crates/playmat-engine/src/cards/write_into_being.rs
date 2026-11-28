//! Write into Being: "Look at the top two cards of your library. Manifest one of those cards, then put the other on
//! the top or bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WriteIntoBeing.WriteIntoBeingEffect", apply)];

/// The top two looked at; the one to manifest chosen (`chooseTarget`, "card to manifest"), or with one card that
/// one (`Cards.getRandom`, which still draws a random number); it's put on top and manifested (`ManifestEffect(1)`);
/// then "Put <the new top card> on bottom of library?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    if !top.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let looked = crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: top.clone() };
        game.looked_at.push(looked);
    }
    let manifest = if top.len() > 1 {
        choose_cards(game, d, you, &top, &top, 1, 1, "card to manifest")?.first().copied()
    } else {
        if top.len() == 1 {
            game.rng.next_int_bound(1);
        }
        top.first().copied()
    };
    // the card to manifest goes on top (the other second)
    if let Some(m) = manifest {
        if game.player(you).library.first() != Some(&m) && game.player(you).library.len() >= 2 {
            let lib = &mut game.player_mut(you).library;
            lib.swap(0, 1);
        }
    }
    cx.run(game, d, &[EffectDef::ManifestEffect { amount: crate::card::Amount::Fixed(1), cloak: false }]);
    if d.stopped() {
        return None;
    }
    let Some(&back) = game.player(you).library.first() else { return Some(()) };
    let name = game.obj(back).def.name.clone();
    if choose_use(game, d, you, &format!("Put {name} on bottom of library?"))? {
        to_library(game, back, false);
    }
    Some(())
}

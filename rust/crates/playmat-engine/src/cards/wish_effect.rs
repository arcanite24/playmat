//! The framework `WishEffect`: "you may put a card you own from outside the game into your hand" (North Wind
//! Avatar, Mastermind's Acquisition), "you may reveal a noncreature card you own from outside the game and put it
//! into your hand" (Fae of Wishes, Legion Angel), and "… or choose a face-up Eldrazi card you own in exile" (Coax
//! from the Blind Eternities). The transpiler gives every constructor the same arguments: the filter, then
//! `alsoFromExile`, `topOfLibrary` and `reveal`. Outside the game is the sideboard the game knows (`outside`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WishEffect", apply).args(&["filter", "text", "text", "text"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let flag = |i: usize| cx.args.get(i).is_some_and(|a| a.trim() == "true");
    let (from_exile, top) = (flag(1), flag(2));
    let message = if top {
        "Put a card you own from outside the game on top of your library?"
    } else {
        "Put a card you own from outside the game into your hand?"
    };
    if !choose_use(game, d, you, message)? {
        return Some(());
    }
    let source = cx.source_id();
    let outside = game.player(you).outside.clone();
    let exiled: Vec<ObjId> = if from_exile {
        game.exile.iter().copied().filter(|c| game.obj(*c).owner == you && !game.obj(*c).face_down).collect()
    } else {
        Vec::new()
    };
    let filter = &cx.filters[0];
    let mut zone = outside.clone();
    zone.extend(exiled.iter().copied());
    let possible: Vec<ObjId> = zone.iter().copied().filter(|c| game.filter_matches(filter, you, Some(source), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &possible, &possible, 1, 1, "Wish")?;
    for c in chosen {
        if top {
            to_library(game, c, true);
        } else {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

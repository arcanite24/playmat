//! Armored Kincaller: "When Armored Kincaller enters the battlefield, you may reveal a Dinosaur card from your hand.
//! If you do or if you control another Dinosaur, you gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArmoredKincaller.ArmoredKincallerEffect", apply)];

/// `TargetCardInHand(0, 1, Dinosaur)` chosen from the hand (not with an empty one), the reveal, then the life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let hand = game.player(you).hand.clone();
    let mut card = None;
    if !hand.is_empty() {
        let dinos: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_subtype(*c, "Dinosaur")).collect();
        card = choose_cards(game, d, you, &hand, &dinos, 0, 1, &item_rule(game, cx.item))?.first().copied();
    }
    if let Some(c) = card {
        let zcc = game.obj(me).zcc;
        game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(c, Zone::Hand)] });
    }
    let other = game
        .battlefield
        .iter()
        .any(|o| *o != me && game.obj(*o).controller == you && game.has_subtype(*o, "Dinosaur"));
    if card.is_some() || other {
        gain_life(game, you, 3);
    }
    Some(())
}

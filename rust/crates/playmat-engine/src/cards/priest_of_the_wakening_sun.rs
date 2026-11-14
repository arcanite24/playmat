//! Priest of the Wakening Sun: "At the beginning of your upkeep, you may reveal a Dinosaur card from your hand. If
//! you do, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PriestOfTheWakeningSun.PriestOfTheWakeningSunEffect", apply)];

/// With a Dinosaur in hand: a yes or no, then `TargetCardInHand(0, 1)` with `chooseTarget`, the reveal and the life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let hand = game.player(you).hand.clone();
    let dinos: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_subtype(*c, "Dinosaur")).collect();
    if dinos.is_empty() {
        return None;
    }
    let q = Query::YesNo { message: "Reveal a Dinosaur card?".into() };
    let yes = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if !yes {
        return None;
    }
    let &c = choose_cards(game, d, you, &hand, &dinos, 0, 1, &item_rule(game, cx.item))?.first()?;
    let zcc = game.obj(me).zcc;
    game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(c, Zone::Hand)] });
    gain_life(game, you, 2);
    Some(())
}

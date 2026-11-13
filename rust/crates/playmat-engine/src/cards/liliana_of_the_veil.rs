//! Liliana of the Veil's −6: "Separate all permanents target player controls into two piles. That player sacrifices
//! all permanents in the pile of their choice."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianaOfTheVeil.LilianaOfTheVeilEffect", apply)];

/// You pick the first pile (`TargetPermanent(0, count)`, one `HumanPlayer.choose`), the rest is the second; the
/// target player answers CHOOSE_PILE (true: the first) and sacrifices that pile's permanents one by one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == p).collect();
    let pile1 = if theirs.is_empty() {
        // `TargetPermanent(0, 0)` is "any number": Java still asks, with nothing to pick
        match ask(game, d, you, &Query::Cards { objs: Vec::new(), required: false }) {
            Answer::Stop => return None,
            _ => Vec::new(),
        }
    } else {
        choose_cards(game, d, you, &theirs, &theirs, 0, theirs.len(), "separate into piles")?
    };
    let pile2: Vec<ObjId> = theirs.iter().copied().filter(|o| !pile1.contains(o)).collect();
    let first = loop {
        match ask(game, d, p, &Query::YesNo { message: "Choose a pile to sacrifice.".into() }) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    for o in if first { pile1 } else { pile2 } {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}

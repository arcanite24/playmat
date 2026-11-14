//! Hostile Negotiations: "Exile the top three cards of your library in a face-down pile, then exile the next top
//! three cards of your library in another face-down pile. Look at the cards in each pile, then turn a pile of your
//! choice face up. An opponent chooses one of those piles. Put all cards in the chosen pile into your hand and the
//! rest into your graveyard. You lose 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HostileNegotiations.HostileNegotiationsEffect", apply)];

/// the top three cards exiled face down, twice; you choose the pile turned face up ("Pile 1" for yes), then the
/// opponent chooses the pile that goes to your hand ("Pile 1" for yes), the other to your graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opp = game.opponent(you);
    let pile1 = exile_top_three(game, you);
    let pile2 = exile_top_three(game, you);
    let first = choose_use(game, d, you, "Choose pile to turn face up")?;
    for c in if first { &pile1 } else { &pile2 } {
        game.obj_mut(*c).face_down = false;
    }
    let to_hand = choose_use(game, d, opp, "Choose pile to go to opponent's hand")?;
    let (hand, grave) = if to_hand { (pile1, pile2) } else { (pile2, pile1) };
    for c in hand {
        game.obj_mut(c).face_down = false;
        if game.obj(c).zone == Zone::Exile {
            game.move_to(c, Zone::Hand);
        }
    }
    for c in grave {
        game.obj_mut(c).face_down = false;
        if game.obj(c).zone == Zone::Exile {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}

/// the top three cards of your library exiled, those now in exile turned face down
fn exile_top_three(game: &mut Game, you: Seat) -> Vec<ObjId> {
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
    }
    let exiled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    for c in &exiled {
        game.obj_mut(*c).face_down = true;
    }
    exiled
}

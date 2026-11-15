//! Riddles in the Dark: "Look at the top four cards of your library and separate them into a face-down pile and a
//! face-up pile. An opponent chooses one of the piles. Put that pile into your hand and the other into your
//! graveyard."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiddlesInTheDark.RiddlesInTheDarkEffect", apply)];

/// You choose up to four of the top four for the face-down pile (`TargetCard(0, 4)`); the face-up pile is revealed.
/// The opponent (the only one; else the one you choose) answers "Choose a pile to put into …'s hand." (yes: the
/// face-down pile); that pile into your hand, the other into your graveyard.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let face_down = choose_cards(game, d, you, &cards, &cards, 0, 4, &rule)?;
    let face_up: Vec<ObjId> = cards.iter().copied().filter(|c| !face_down.contains(c)).collect();
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    let opponent = match opponents.as_slice() {
        [] => return Some(()),
        [p] => *p,
        _ => loop {
            let q = Query::Target { players: opponents.clone(), objs: Vec::new(), required: true };
            match ask(game, d, you, &q) {
                Answer::Player(p) if opponents.contains(&p) => break p,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        },
    };
    let message = format!("Choose a pile to put into {}'s hand.", log::player(you));
    let (to_hand, to_graveyard) =
        if choose_use(game, d, opponent, &message)? { (face_down, face_up) } else { (face_up, face_down) };
    if !put_cards(game, d, you, &to_hand, PutCards::Hand) || !put_cards(game, d, you, &to_graveyard, PutCards::Graveyard)
    {
        return None;
    }
    Some(())
}

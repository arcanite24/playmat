//! Memories Returning: "Reveal the top five cards of your library. Put one of them into your hand. Then choose an
//! opponent. They put one on the bottom of your library. Then you put one into your hand. Then they put one on the
//! bottom of your library. Put the other into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MemoriesReturning.MemoriesReturningEffect", apply)];

/// The five revealed; then turns (the opponent chosen after the first), you to the hand and the opponent to the bottom of your library, each a
/// `Player.choose` of one card from those left (a last card goes without a choice), until none are left
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: cards.iter().map(|c| (*c, Zone::Library)).collect(),
    });
    let mut opponent = game.opponent(you);
    let rule = item_rule(game, cx.item);
    for step in 0..5 {
        cards.retain(|c| game.obj(*c).zone == Zone::Library);
        if cards.is_empty() {
            break;
        }
        // after the first card: "choose an opponent" (`TargetOpponent`, not targeted, with `Player.choose`: asked
        // even with one opponent)
        if step == 1 {
            let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
            loop {
                match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
                    Answer::Player(p) if opponents.contains(&p) => {
                        opponent = p;
                        break;
                    }
                    Answer::Stop => return None,
                    _ => {}
                }
            }
        }
        let to_hand = step % 2 == 0;
        let chosen = if cards.len() == 1 {
            cards[0]
        } else {
            let who = if to_hand { you } else { opponent };
            *choose_cards(game, d, who, &cards, &cards, 1, 1, &rule)?.first()?
        };
        if to_hand {
            game.move_to(chosen, Zone::Hand);
        } else {
            to_library(game, chosen, false);
        }
        if cards.len() == 1 {
            break;
        }
    }
    Some(())
}

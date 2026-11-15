//! Thought Distortion: "Target opponent reveals their hand. Exile all noncreature, nonland cards from that player's
//! hand and graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThoughtDistortion.ThoughtDistortionEffect", apply)];

/// the hand revealed, then the hand's and the graveyard's noncreature, nonland cards exiled together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let fits = |g: &Game, c: ObjId| !g.obj(c).is(CardType::Creature) && !g.obj(c).is(CardType::Land);
    let mut cards: Vec<ObjId> = hand.into_iter().filter(|c| fits(game, *c)).collect();
    cards.extend(game.player(p).graveyard.iter().copied().filter(|c| fits(game, *c)));
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}

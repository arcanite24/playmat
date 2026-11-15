//! Whispering Madness: "Each player discards their hand, then draws cards equal to the greatest number of cards a
//! player discarded this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhisperingMadness.WhisperingMadnessEffect", whispering_madness)];

/// each player from you discards their hand, then each draws the most any of them discarded
fn whispering_madness(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let seats = game.seats();
    let order: Vec<Seat> =
        (0..seats).map(|k| (cx.controller() + k) % seats).filter(|p| !game.player(*p).lost).collect();
    let mut most = 0;
    for p in &order {
        let hand = game.player(*p).hand.clone();
        let mut n = 0;
        for c in hand {
            discard_card(game, *p, c);
            n += usize::from(game.obj(c).zone == Zone::Graveyard);
        }
        most = most.max(n);
    }
    for p in order {
        draw(game, p, most);
    }
    Some(())
}

//! Baleful Mastery: "You may pay {1}{B} rather than pay this spell's mana cost. If the {1}{B} cost was paid, an
//! opponent draws a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BalefulMastery.BalefulMasteryAlternativeCostEffect", apply)];

/// when the spell was cast for its alternative cost: an opponent you choose (`chooseTarget` of a not-targeted
/// `TargetOpponent`, asked even with one) draws a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !game.obj(cx.source_id()).alternative_paid {
        return Some(());
    }
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    if opponents.is_empty() {
        return Some(());
    }
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    draw(game, opponent, 1);
    Some(())
}

//! Devastating Mastery: "You may pay {2}{W}{W} rather than pay this spell's mana cost. If the {2}{W}{W} cost was paid,
//! an opponent chooses up to two nonland permanents they control and returns them to their owner's hand. Destroy all
//! nonland permanents."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DevastatingMastery.DevastatingMasteryAlternativeCostEffect", apply)];

/// cast for its alternative cost: an opponent you choose (`chooseTarget` of a not-targeted `TargetOpponent`)
/// chooses up to two nonland permanents (Java's filter, `FILTER_PERMANENTS_NON_LAND`, isn't only theirs) and they go
/// to their owners' hands
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
    let nonland: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !game.has_type(*o, CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, opponent, &nonland, &nonland, 0, 2, &rule)?;
    put_cards(game, d, opponent, &chosen, crate::card::PutCards::Hand);
    Some(())
}

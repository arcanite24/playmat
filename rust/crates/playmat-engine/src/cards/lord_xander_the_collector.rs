//! Lord Xander, the Collector: "When Lord Xander enters, target opponent discards half the cards in their hand,
//! rounded down. Whenever Lord Xander attacks, defending player mills half their library, rounded down. When Lord
//! Xander dies, target opponent sacrifices half the nonland permanents they control, rounded down."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LordXanderTheCollector.LordXanderTheCollectorEffectType.DISCARD", discard_half),
    Handler::effect("LordXanderTheCollector.LordXanderTheCollectorEffectType.MILL", mill_half),
    Handler::effect("LordXanderTheCollector.LordXanderTheCollectorEffectType.SACRIFICE", sacrifice_half),
];

/// The target opponent (with two or more cards in hand) discards half, rounded down
fn discard_half(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let count = game.player(p).hand.len() as i32;
    if count >= 2 {
        discard(game, d, p, count / 2);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

/// The defending player (the trigger's player pointer, with two or more cards in library) mills half
fn mill_half(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let count = game.player(p).library.len();
    if count >= 2 {
        let milled: Vec<ObjId> = game.player(p).library.iter().take(count / 2).copied().collect();
        note(game, EventKind::Milled, Some(p), None, None, milled.len() as i32);
        for c in milled {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}

/// The target opponent (with two or more nonland permanents) chooses half of them, not targeted, and sacrifices
/// those
fn sacrifice_half(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land))
        .collect();
    if theirs.len() < 2 {
        return Some(());
    }
    let n = theirs.len() / 2;
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, p, &theirs, &theirs, n, n, &rule)?;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}

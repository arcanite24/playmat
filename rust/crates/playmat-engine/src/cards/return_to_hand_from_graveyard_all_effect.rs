//! `ReturnToHandFromGraveyardAllEffect(filter[, targetController])` (Wisdom of Ages): "each player returns each
//! <filter> from their graveyard to their hand" (or only you).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnToHandFromGraveyardAllEffect", apply).args(&[
    "filter",
    "one_of:TargetController.YOU|TargetController.EACH_PLAYER|TargetController.ANY|TargetController.OPPONENT",
])];

/// The players from you in turn order (Java's switch: OPPONENT falls through to YOU's test, so it returns nothing),
/// each one's matching graveyard cards in graveyard order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let who = cx.args.get(1).map(String::as_str).unwrap_or("TargetController.EACH_PLAYER");
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        match who {
            "TargetController.OPPONENT" => continue,
            "TargetController.YOU" if p != you => continue,
            _ => {}
        }
        let cards: Vec<ObjId> = game
            .player(p)
            .graveyard
            .iter()
            .copied()
            .filter(|c| game.filter_matches(&cx.filters[0], p, Some(cx.source_id()), *c))
            .collect();
        for c in cards {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

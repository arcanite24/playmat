//! Gideon's Triumph: "Target opponent sacrifices a creature that attacked or blocked this turn. If you control a
//! Gideon planeswalker, that player sacrifices two of those creatures instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GideonsTriumph.GideonsTriumphEffect", apply),
    Handler::predicate("GideonsTriumph.GideonsTriumphPredicate", attacked_or_blocked),
];

/// `SacrificeEffect(filter, 1 or 2, "Target opponent")` on the target opponent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let gideon = game.battlefield.iter().any(|o| {
        game.obj(*o).controller == you && game.has_type(*o, CardType::Planeswalker) && game.has_subtype(*o, "Gideon")
    });
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "SacrificeEffect", "amount": if gideon { 2 } else { 1 },
        "filter": [{"p": "card", "handler": "GideonsTriumph.GideonsTriumphPredicate"}]
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}

/// a creature that attacked or blocked this turn (`GideonsTriumphWatcher`), as the object it is now
fn attacked_or_blocked(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let at = (id, game.obj(id).zcc);
    game.has_type(id, CardType::Creature) && (game.attacked_this_turn.contains(&at) || game.watch.blocked.contains(&at))
}

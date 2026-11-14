//! Gideon's Sacrifice: "Choose a creature or planeswalker you control. All damage that would be dealt this turn to
//! you and permanents you control is dealt to the chosen permanent instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "GideonsSacrifice.GideonsSacrificeEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.on_start(choose)];

/// `player.choose(TargetPermanent(filter).withNotTarget(true))`: a creature or planeswalker you control, kept as a
/// `MageObjectReference` (nothing chosen: no effect). `HumanPlayer.choose` asks even with one to choose from.
fn choose(game: &mut Game, d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == cx.controller
                && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
        })
        .collect();
    if can.is_empty() {
        return Some(Vec::new());
    }
    loop {
        match ask(game, d, cx.controller, &Query::Cards { objs: can.clone(), required: true }) {
            Answer::Obj(o) if can.contains(&o) => return Some(vec![(Target::Obj(o), game.obj(o).zcc)]),
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

/// damage to you, or to a permanent you control, once a permanent was chosen
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if cx.targets.is_empty() {
        return false;
    }
    match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| game.obj(t).zone == Zone::Battlefield && game.obj(t).controller == cx.controller),
    }
}

/// dealt to the chosen permanent instead, while it's still that permanent
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let Some(chosen) = cx.target_obj(game, 0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return false };
    redirect_damage(game, ev, chosen);
    true
}

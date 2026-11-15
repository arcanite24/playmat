//! Urabrask // The Great Work: "First strike. Whenever you cast an instant or sorcery spell, Urabrask deals 1 damage
//! to target opponent. Add {R}. {R}: Exile Urabrask, then return it to the battlefield transformed under its owner's
//! control. Activate only as a sorcery and only if you've cast three or more instant and/or sorcery spells this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("Urabrask.UrabraskCondition", three_instants_or_sorceries),
    Handler::effect("Urabrask.TheGreatWorkEffect", great_work),
    Handler::as_though("Urabrask.TheGreatWorkCastFromGraveyardEffect", AsThough::Play(cast_from_graveyard)),
    Handler::replacement(
        "Urabrask.TheGreatWorkReplacementEffect",
        &[EventType::ZoneChange],
        exiles_applies,
        exiles_replace,
    ),
];

/// Chapter III, until end of turn: you may cast instant and sorcery cards from any graveyard, for their own costs
fn cast_from_graveyard(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> Option<PlayWay> {
    (seat == cx.controller
        && game.obj(id).zone == Zone::Graveyard
        && (face.is(CardType::Instant) || face.is(CardType::Sorcery)))
    .then(PlayWay::default)
}

/// An instant or sorcery spell cast from a graveyard (`CastFromGraveyardWatcher`, while it's still that spell) would
/// be put into a graveyard
fn exiles_applies(game: &Game, _: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    let o = game.obj(id);
    ev.to == Some(Zone::Graveyard)
        && ev.from == Some(Zone::Stack)
        && o.cast_from == Some(Zone::Graveyard)
        && (o.def.is(CardType::Instant) || o.def.is(CardType::Sorcery))
}

/// … it's exiled instead
fn exiles_replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}

/// The Great Work's chapter I: 3 damage to the target opponent, then 3 to each creature they control, in
/// battlefield order
fn great_work(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    deal_damage(game, cx.source_id(), Target::Player(p), 3, false);
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        if game.obj(o).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(o), 3, false);
        }
    }
    Some(())
}

/// You've cast three or more instant and/or sorcery spells this turn (`SpellsCastWatcher`)
fn three_instants_or_sorceries(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch
        .spells
        .iter()
        .filter(|s| {
            s.caster == you && {
                let def = &game.obj(s.card).def;
                def.is(CardType::Instant) || def.is(CardType::Sorcery)
            }
        })
        .count()
        >= 3
}

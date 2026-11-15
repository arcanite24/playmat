//! Gisa, Glorious Resurrector: "If a creature an opponent controls would die, exile it instead. At the beginning of
//! your upkeep, put all creature cards exiled with Gisa onto the battlefield under your control. They gain decayed."

use super::*;
use crate::state::Grant;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "GisaGloriousResurrector.GisaGloriousResurrectorExileEffect",
        &[EventType::ZoneChange],
        applies,
        replace,
    ),
    Handler::effect("GisaGloriousResurrector.GisaGloriousResurrectorReturnEffect", apply),
];

/// A dies event (battlefield to a graveyard) of a creature controlled by an opponent of Gisa's controller
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield)
        && ev.to == Some(Zone::Graveyard)
        && game.has_type(id, CardType::Creature)
        && ev.player != cx.controller
}

/// Exiled instead: a token just to exile, a card into Gisa's exile zone (as Gisa is now)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    if let Some(id) = ev.target {
        if !game.obj(id).token {
            let zcc = game.obj(cx.source).zcc;
            game.exiled_for_source.push((cx.source, zcc, id));
        }
    }
    false
}

/// The creature cards in Gisa's exile zone (as Gisa is now), in the order they went there, onto the battlefield
/// under your control; each that's there gains decayed for as long as it stays (`Duration.Custom`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let zcc = game.obj(src).zcc;
    let mut cards: Vec<ObjId> = Vec::new();
    for (s, z, c) in game.exiled_for_source.iter().copied() {
        if s == src
            && z == zcc
            && game.obj(c).zone == Zone::Exile
            && game.obj(c).is(CardType::Creature)
            && !cards.contains(&c)
        {
            cards.push(c);
        }
    }
    if cards.is_empty() {
        return Some(());
    }
    game.exiled_for_source.retain(|(s, z, c)| !(*s == src && *z == zcc && cards.contains(c)));
    for c in &cards {
        put_onto_battlefield(game, d, *c, you);
        if d.stopped() {
            return None;
        }
    }
    for c in cards {
        if game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        let zcc = game.obj(c).zcc;
        Arc::make_mut(&mut game.grants).push(Grant {
            target: c,
            zcc,
            ability: AbilityDef::DecayedAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
    }
    Some(())
}

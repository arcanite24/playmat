//! Glamdring, Foe-hammer: "Instant and sorcery spells you cast cost {X} less to cast, where X is equipped creature's
//! power." Gleam of Death: "Mill six cards, then put all instant and sorcery cards from among them into your hand."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::cost("GlamdringFoeHammer.GlamdringFoeHammerEffect", CostKind::Reduce, instant_or_sorcery, less_by_power),
    Handler::effect("GlamdringFoeHammer.GleamofDeathEffect", gleam_of_death),
];

/// instant and sorcery spells you cast
fn instant_or_sorcery(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && (of.face.is(CardType::Instant) || of.face.is(CardType::Sorcery))
}

/// `CardUtil.reduceCost` by the equipped creature's power (none, or a negative power: nothing)
fn less_by_power(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let Some(creature) = game.attached(cx.source) else { return };
    let x = game.power_toughness(creature).0.max(0) as u32;
    cost.generic = cost.generic.saturating_sub(x);
}

/// six milled; the instant and sorcery cards among them still in the graveyard go to the hand
fn gleam_of_death(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    for c in top {
        let o = game.obj(c);
        if o.zone == Zone::Graveyard && (o.is(CardType::Instant) || o.is(CardType::Sorcery)) {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}

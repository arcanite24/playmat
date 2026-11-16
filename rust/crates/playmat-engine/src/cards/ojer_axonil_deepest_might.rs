//! Ojer Axonil, Deepest Might // Temple of Power. Ojer Axonil: "If a red source you control would deal an amount of
//! noncombat damage less than Ojer Axonil's power to an opponent, that source deals damage equal to Ojer Axonil's
//! power instead. When Ojer Axonil dies, return it to the battlefield tapped and transformed under its owner's
//! control." Temple of Power: "{2}{R}, {T}: Transform Temple of Power. Activate only if red sources you controlled
//! dealt 4 or more noncombat damage this turn and only as a sorcery."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "OjerAxonilDeepestMight.OjerAxonilDeepestMightReplacementEffect",
        &[EventType::DamagePlayer],
        applies,
        replace,
    ),
    Handler::effect("OjerAxonilDeepestMight.OjerAxonilDeepestMightTransformEffect", return_transformed),
    Handler::condition("OjerAxonilDeepestMight.TempleOfPowerCondition", four_red_noncombat),
];

/// Ojer's power while it's still on the battlefield
fn ojer_power(game: &Game, cx: &StaticCx) -> Option<i32> {
    (game.obj(cx.source).zone == Zone::Battlefield).then(|| game.power_toughness(cx.source).0)
}

/// noncombat damage to an opponent from a red source you control, less than Ojer's power
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    let Some(power) = ojer_power(game, cx) else { return false };
    ev.player != cx.controller
        && game.obj(src).controller == cx.controller
        && game.colors(src).contains(&Color::R)
        && !ev.combat
        && ev.amount > 0
        && ev.amount < power
}

/// that source deals damage equal to Ojer's power instead
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(power) = ojer_power(game, cx).filter(|p| *p > 0) {
        ev.amount = power;
    }
    false
}

/// the card, still in the graveyard it died to, returns to the battlefield transformed and tapped under its owner's
/// control (`moveCards(card, BATTLEFIELD, tapped, byOwner)` with the enter-transformed value)
fn return_transformed(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let Some(back) = game.obj(card).def.transform_back.clone() else { return Some(()) };
    let owner = game.obj(card).owner;
    let o = game.obj_mut(card);
    o.front = Some(std::mem::replace(&mut o.def, back));
    enter_battlefield(game, d, card, owner, false);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).tapped = true;
    }
    Some(())
}

/// `TempleOfPowerWatcher`: the noncombat damage red sources you controlled dealt this turn, 4 or more
fn four_red_noncombat(game: &Game, you: Seat, _: ObjId) -> bool {
    let n: i32 = game
        .watch
        .damage
        .iter()
        .filter(|d| !d.combat && d.source_controller == you && game.colors(d.source).contains(&Color::R))
        .map(|d| d.amount)
        .sum();
    n >= 4
}

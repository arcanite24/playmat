//! Invasion of Regatha // Disciples of the Inferno: "If a noncreature source you control would deal damage to a
//! creature, battle, or opponent, it deals that much damage plus 2 instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "InvasionOfRegatha.DisciplesOfTheInfernoEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    plus_two,
)];

/// Java's `applies`: damage to an opponent of yours, or to a creature or battle, from a source that isn't a creature
/// (as it last was on the battlefield, or as the object it is now); its text's "you control" isn't checked
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let to_ok = match ev.kind {
        EventType::DamagePlayer => ev.player != cx.controller,
        _ => ev.target.is_some_and(|t| {
            game.obj(t).zone == Zone::Battlefield
                && (game.has_type(t, CardType::Creature) || game.has_type(t, CardType::Battle))
        }),
    };
    let Some(s) = ev.source else { return false };
    let o = game.obj(s);
    let creature = match o.zone {
        Zone::Battlefield => game.has_type(s, CardType::Creature),
        Zone::Graveyard | Zone::Exile => o.lki_def.as_ref().unwrap_or(&o.def).is(CardType::Creature),
        _ => o.def.is(CardType::Creature),
    };
    to_ok && !creature && ev.amount > 0
}

/// `CardUtil.overflowInc(amount, 2)`
fn plus_two(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(2);
    false
}

//! Avacyn, Guardian Angel: "{1}{W}: Prevent all damage that would be dealt to another target creature this turn by
//! sources of the color of your choice. {5}{W}{W}: Prevent all damage that would be dealt to target player or
//! planeswalker this turn by sources of the color of your choice."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "AvacynGuardianAngel.AvacynGuardianAngelPreventToCreatureEffect",
        &[EventType::DamagePermanent],
        to_target,
        prevent_all,
    )
    .on_start(choose_color),
    Handler::replacement(
        "AvacynGuardianAngel.AvacynGuardianAngelPreventToPlayerEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        to_target,
        prevent_all,
    )
    .on_start(choose_color),
];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// `ChoiceColor` ("Choose color"), kept as the effect's amount (its index in WUBRG), pointed at the ability's target
fn choose_color(game: &mut Game, d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let choices: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Choose color".into(), choices };
    let k = loop {
        match ask(game, d, cx.controller, &q) {
            Answer::Text(t) => {
                if let Some(k) = NAMES.iter().position(|n| *n == t) {
                    break k;
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    cx.set_left(game, k as i32);
    Some(cx.targets.to_vec())
}

/// preventable damage to the target (a creature or planeswalker still that permanent, or the player) from a source
/// that shares the chosen color
fn to_target(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) {
        return false;
    }
    let hit = match ev.kind {
        EventType::DamagePermanent => ev.target.is_some() && ev.target == cx.target_obj(game, 0),
        _ => cx.target_player(0) == Some(ev.player),
    };
    let color = Color::ALL[cx.left(game, 0).clamp(0, 4) as usize];
    hit && ev.source.is_some_and(|s| game.colors(s).contains(&color))
}

fn prevent_all(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}

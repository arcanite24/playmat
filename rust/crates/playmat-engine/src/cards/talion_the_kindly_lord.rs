//! Talion, the Kindly Lord: "As Talion, the Kindly Lord enters, choose a number between 1 and 10. Whenever an
//! opponent casts a spell with mana value, power, or toughness equal to the chosen number, that player loses 2 life
//! and you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TalionTheKindlyLord.TalionTheKindlyLordEffect", choose_number),
    Handler::predicate("TalionTheKindlyLord.TalionTheKindlyLordPredicate", equal_to_number),
];

/// the number chosen as `id` entered, kept as one of its marks while it stays
fn chosen_number(game: &Game, id: ObjId) -> Option<i32> {
    game.obj(id).marks.iter().find_map(|m| m.strip_prefix("chosen number ")?.parse().ok())
}

/// `getAmount(1, 10, "Choose a number.")`, kept on the entering permanent with its info line and the log's
fn choose_number(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    let n = loop {
        match ask(game, d, cx.controller(), &Query::Amount { min: 1, max: 10 }) {
            Answer::Int(n) if (1..=10).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let o = game.obj_mut(id);
    o.marks.retain(|m| !m.starts_with("chosen number "));
    o.marks.push(format!("chosen number {n}"));
    o.add_info("chosen players", &format!("Chosen Number: {n}"));
    if log::kept(game) {
        let line = format!("{}, chosen number: {}", log::object(game, id), n);
        log::inform(game, line);
    }
    Some(())
}

/// the spell's mana value (its X included), power or toughness is the number Talion's source chose
fn equal_to_number(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(n) = source.and_then(|s| chosen_number(game, s)) else { return false };
    let def = game.obj(id).def.clone();
    let x = game.stack.iter().find(|i| i.card == id && i.ability.is_none()).map_or(0, |i| i.x);
    let mv = def.mana_value() as i32 + x * def.printed().x as i32;
    let (p, t) = game.power_toughness(id);
    mv == n || p == n || t == n
}

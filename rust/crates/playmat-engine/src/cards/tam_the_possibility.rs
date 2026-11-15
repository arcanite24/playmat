//! Tam, the Possibility: "Planeswalker spells you cast cost {1} less to cast. {W}{U}{B}{R}{G}, {T}: Proliferate X
//! times, where X is the number of planeswalker types among planeswalkers you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TamThePossibility.TamThePossibilityEffect", apply).effects(&["new ProliferateEffect()"])];

/// X as it starts (`TamThePossibilityValue`: the distinct planeswalker types of the planeswalkers you control, read
/// from their cards' subtypes), then `ProliferateEffect` X times
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut types: Vec<String> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller != you || !game.has_type(o, CardType::Planeswalker) {
            continue;
        }
        for s in &game.obj(o).def.subtypes {
            if crate::card::NON_CREATURE_TYPES.contains(&s.as_str()) && !types.contains(s) {
                types.push(s.clone());
            }
        }
    }
    for _ in 0..types.len() {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

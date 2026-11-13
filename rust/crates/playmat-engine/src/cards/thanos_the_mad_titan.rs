//! Thanos, the Mad Titan: "Power-up — {C}{W}{U}{B}{R}{G}: Put two +1/+1 counters on Thanos. Choose odd or even.
//! Destroy each other creature with mana value of the chosen quality."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThanosTheMadTitan.ThanosTheMadTitanEffect", apply)];

/// With Thanos still the permanent: you choose odd or even (`ChoiceImpl`, required); each other creature whose mana
/// value has that parity is destroyed, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let thanos = cx.source_id();
    let choices: Vec<String> = vec!["odd".into(), "even".into()];
    let q = Query::Choice { message: "Choose odd or even".into(), choices: choices.clone() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) if choices.contains(&t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let odd = chosen == "odd";
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            *o != thanos && game.has_type(*o, CardType::Creature) && odd == (game.obj(*o).def.mana_value() % 2 == 1)
        })
        .collect();
    for o in doomed {
        if game.obj(o).zone == Zone::Battlefield {
            destroy(game, o);
        }
    }
    Some(())
}

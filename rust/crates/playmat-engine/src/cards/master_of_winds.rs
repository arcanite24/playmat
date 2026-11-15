//! Master of Winds: "Whenever you cast an instant, sorcery, or Wizard spell, you may have Master of Winds's base power
//! and toughness become 4/1 or 1/4 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MasterOfWinds.MasterOfWindsEffect", apply).effects(&[
    "new SetBasePowerToughnessSourceEffect(4, 1, Duration.EndOfTurn)",
    "new SetBasePowerToughnessSourceEffect(1, 4, Duration.EndOfTurn)",
])];

/// "Have this creature become a 4/1 or a 1/4?" (yes: 4/1), then that base until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::YesNo { message: "Have this creature become a 4/1 or a 1/4?".into() };
    let four = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    let i = if four { 0 } else { 1 };
    cx.run(game, d, &cx.effects[i..=i]);
    Some(())
}

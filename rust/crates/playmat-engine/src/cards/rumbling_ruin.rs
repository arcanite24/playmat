//! Rumbling Ruin: "When this creature enters, count the number of +1/+1 counters on creatures you control. Creatures
//! your opponents control with power less than or equal to that number can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RumblingRuin.RumblingRuinEffect", apply)];

/// 1 plus the +1/+1 counters on creatures you control; `CantBlockAllEffect` until end of turn for opponents'
/// creatures with power less than that
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n: i32 = 1 + game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.obj(*o).counter("+1/+1"))
        .sum::<i32>();
    let filter: Filter = serde_json::from_value(serde_json::json!([
        {"p": "type", "t": "CREATURE"},
        {"p": "controller", "who": "opponent"},
        {"cmp": "FEWER_THAN", "n": n, "p": "power"}
    ]))
    .ok()?;
    let e = EffectDef::CantBlockAllEffect { filter, duration: crate::card::Duration::EndOfTurn };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}

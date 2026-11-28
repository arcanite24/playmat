//! Ocelot Pride: "At the beginning of your end step, if you gained life this turn, create a 1/1 white Cat creature
//! token. Then if you have the city's blessing, for each token you control that entered the battlefield this turn,
//! create a token that's a copy of it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OcelotPride.OcelotPrideEffect", apply)
    .effects(&["new CreateTokenCopyTargetEffect()"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let turn = game.turn;
    let tokens: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let obj = game.obj(*o);
            obj.controller == you && obj.token && obj.entered_turn == turn
        })
        .collect();
    for tok in tokens {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(tok)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

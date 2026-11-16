//! Grakmaw, Skyclave Ravager: "Grakmaw enters the battlefield with three +1/+1 counters on it. Whenever another
//! creature you control dies, if it had a +1/+1 counter on it, put a +1/+1 counter on Grakmaw. When Grakmaw dies,
//! create an X/X black and green Hydra creature token, where X is the number of +1/+1 counters on Grakmaw."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GrakmawSkyclaveRavager.GrakmawSkyclaveRavagerEffect", apply)
    .tokens(&["GrakmawSkyclaveRavagerHydraToken"])];

/// `new GrakmawSkyclaveRavagerHydraToken(x)`, X the +1/+1 counters Grakmaw had as it left the battlefield
/// (`permanentLeftBattlefield`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = game.obj(cx.source_id()).lki_counters.iter().find(|(n, _)| n == "+1/+1").map_or(0, |(_, k)| *k);
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(x) };
    cx.run_on(game, d, &[e], Vec::new());
    (!d.stopped()).then_some(())
}

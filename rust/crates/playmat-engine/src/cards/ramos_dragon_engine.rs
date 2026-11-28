//! Ramos, Dragon Engine: "Whenever you cast a spell, put a +1/+1 counter on Ramos for each of that spell's colors."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RamosDragonEngine.RamosDragonEngineAddCountersEffect", apply)];

/// The spell the trigger points at (on the stack or as it last was there): one +1/+1 counter per color, on Ramos
/// while it's on the battlefield (`AddCountersSourceEffect` with that many)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let Some(Target::Obj(spell)) = cx.item.targets.first().and_then(|t| t.first()).copied() else { return Some(()) };
    let n = game.colors(spell).len() as i32;
    if n > 0 {
        let counter = crate::card::CounterDef { name: "+1/+1".into(), amount: n };
        cx.run(game, d, &[EffectDef::AddCountersSourceEffect { counter, amount: None }]);
    }
    Some(())
}

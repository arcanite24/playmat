//! Power Plant Worker: "{3}: Power Plant Worker gets +2/+2 until end of turn. If you control creatures named Mine
//! Worker and Tower Worker, put two +1/+1 counters on Power Plant Worker instead. Activate only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PowerPlantWorker.PowerPlantWorkerEffect", apply)];

/// the source still on the battlefield: two +1/+1 counters with a Mine Worker and a Tower Worker among the
/// creatures you control, else +2/+2 until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let src = cx.source_id();
    let names: Vec<String> = controlled_creatures(game, you).iter().map(|c| game.obj(*c).def.name.clone()).collect();
    if names.iter().any(|n| n == "Mine Worker") && names.iter().any(|n| n == "Tower Worker") {
        add_counters_by(game, src, "+1/+1", 2, Some(you));
        return Some(());
    }
    let e = EffectDef::BoostSourceEffect {
        power: crate::card::Amount::Fixed(2),
        toughness: crate::card::Amount::Fixed(2),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}

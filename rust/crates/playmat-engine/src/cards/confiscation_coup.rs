//! Confiscation Coup: "Choose target artifact or creature. You get {E}{E}{E}{E}, then you may pay an amount of {E}
//! equal to that permanent's mana value. If you do, gain control of it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConfiscationCoup.ConfiscationCoupEffect", apply)];

/// Four energy; then, with the target still there and energy enough (`PayEnergyCost.canPay`), you're asked, and
/// paying gains control of it for the rest of the game (`GainControlTargetEffect(Duration.Custom)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    add_energy(game, you, 4);
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let mv = game.obj(o).def.mana_value() as i32;
    if energy(game, you) < mv {
        return Some(());
    }
    let message = format!("Pay {} to get control of {}?", "{E}".repeat(mv as usize), game.obj(o).name());
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    if mv > 0 {
        add_energy(game, you, -mv);
    }
    let e: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "GainControlTargetEffect", "duration": "Forever"})).ok()?;
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    Some(())
}

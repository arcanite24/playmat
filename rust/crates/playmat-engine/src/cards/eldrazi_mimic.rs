//! Eldrazi Mimic: "Whenever another colorless creature you control enters, you may have the base power and
//! toughness of Eldrazi Mimic become that creature's power and toughness until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EldraziMimic.EldraziMimicEffect", apply)];

/// That creature's power and toughness now become the Mimic's base power and toughness until end of turn
/// (`SetBasePowerToughnessTargetEffect` at the source)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let (p, t) = game.power_toughness(o);
    let e = EffectDef::SetBasePowerToughnessTargetEffect {
        power: Amount::Fixed(p),
        toughness: Amount::Fixed(t),
        permanent: false,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(cx.source_id())]);
    Some(())
}

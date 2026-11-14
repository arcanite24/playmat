//! Exuberant Wolfbear: "Whenever Exuberant Wolfbear attacks, you may change the base power and toughness of target
//! Human you control to Exuberant Wolfbear's power and toughness until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExuberantWolfbear.ExuberantWolfbearEffect", apply)];

/// the Wolfbear's power and toughness as it resolves, locked in (`SetBasePowerToughnessTargetEffect(p, t, EndOfTurn)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield || game.obj(src).zcc != cx.source.1 {
        return Some(());
    }
    let (p, t) = game.power_toughness(src);
    let e = EffectDef::SetBasePowerToughnessTargetEffect {
        power: Amount::Fixed(p),
        toughness: Amount::Fixed(t),
        permanent: false,
    };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}

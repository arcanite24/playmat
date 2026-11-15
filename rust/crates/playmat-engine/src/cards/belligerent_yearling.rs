//! Belligerent Yearling: "Trample. Whenever another Dinosaur you control enters, you may have this creature's base
//! power become equal to that creature's power until end of turn."

use super::*;
use crate::card::{Amount, Duration};

pub const HANDLERS: &[Handler] = &[Handler::effect("BelligerentYearling.BelligerentYearlingEffect", apply)];

/// That creature (or as it last was): `SetBasePowerSourceEffect(its power, EndOfTurn)` on the source
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(it) = cx.obj(0).or(cx.item.entering) else { return Some(()) };
    let power = game.power_toughness(it).0;
    let e = EffectDef::SetBasePtEffect {
        power: Some(Amount::Fixed(power)),
        toughness: None,
        target: false,
        duration: Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    Some(())
}

//! Unnatural Moonrise: "It becomes night. Until end of turn, target creature gets +1/+0 and gains trample and …"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnnaturalMoonrise.UnnaturalMoonriseEffect", apply)];

/// `game.setDaytime(false)`
fn apply(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    game.day = Some(false);
    Some(())
}

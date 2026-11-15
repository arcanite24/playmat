//! Railway Brawler: "Reach, trample. Whenever another creature you control enters, put X +1/+1 counters on it, where
//! X is its power. Plot {3}{G}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RailwayBrawler.RailwayBrawlerEffect", apply)];

/// the creature that entered, still on the battlefield with power 1 or more: that many +1/+1 counters
/// (`AddCountersTargetEffect`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.obj(0)?;
    if game.obj(id).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(id).0;
    if power > 0 {
        add_counters_by(game, id, "+1/+1", power, Some(cx.controller()));
    }
    Some(())
}

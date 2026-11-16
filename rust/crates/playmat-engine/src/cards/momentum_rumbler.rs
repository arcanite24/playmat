//! Momentum Rumbler: "Whenever Momentum Rumbler attacks, if it doesn't have first strike, put a first strike counter
//! on it. Whenever Momentum Rumbler attacks, if it has first strike, it gains double strike until end of turn."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("MomentumRumbler.MomentumRumblerCondition.TRUE", has_first_strike),
    Handler::condition("MomentumRumbler.MomentumRumblerCondition.FALSE", no_first_strike),
];

/// the Rumbler (as it last was on the battlefield) has first strike
fn has_first_strike(game: &Game, _: Seat, source: ObjId) -> bool {
    game.has_ability(source, &AbilityDef::FirstStrikeAbility)
}

fn no_first_strike(game: &Game, _: Seat, source: ObjId) -> bool {
    !game.has_ability(source, &AbilityDef::FirstStrikeAbility)
}

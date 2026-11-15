//! Raffine's Silencer: "When this creature dies, target creature an opponent controls gets -X/-X until end of turn,
//! where X is this creature's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("RaffinesSilencer.RaffinesSilencerValue", minus_power)];

/// minus the source's power as it last was on the battlefield (`getValue("permanentLeftBattlefield")`)
fn minus_power(game: &Game, _: Seat, source: ObjId, _: i32) -> i32 {
    let card = game.resolving_item.as_ref().map_or(source, |i| i.card);
    -game.obj(card).power_lki.unwrap_or(0)
}

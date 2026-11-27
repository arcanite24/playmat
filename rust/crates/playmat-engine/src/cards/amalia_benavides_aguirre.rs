//! Amalia Benavides Aguirre: "Whenever you gain life, Amalia Benavides Aguirre explores. Then destroy all other
//! creatures if its power is exactly 20."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("AmaliaBenavidesAguirre.AmaliaBenavidesAguirreCondition", power_is_20)];

fn power_is_20(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let power = if o.zone == Zone::Battlefield { game.power_toughness(source).0 } else { o.def.power };
    power == 20
}

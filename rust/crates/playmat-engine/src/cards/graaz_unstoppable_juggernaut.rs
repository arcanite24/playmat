//! Graaz, Unstoppable Juggernaut: "Other creatures you control have base power and toughness 5/3 and are Juggernauts
//! in addition to their other creature types." The subtype part, an `AddCardSubtypeAllEffect(filter, JUGGERNAUT,
//! null)` (layer 4), the filter looked at each time.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "GraazUnstoppableJuggernaut.AddCardSubtypeAllEffect",
    affects,
    Layer::Type(juggernaut),
)
.args(&["filter", "one_of:SubType.JUGGERNAUT", "one_of:null"])];

fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.matches(game, 0, id)
}

fn juggernaut(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    vec!["Juggernaut".into()]
}

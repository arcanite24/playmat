//! Cavernous Maw: "{T}: Add {C}. {2}: Cavernous Maw becomes a 3/3 Elemental creature until end of turn. It's still a
//! land. Activate only if the number of other Caves you control plus the number of Cave cards in your graveyard is
//! three or greater."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CavernousMaw.CavernousMawCondition", three_caves)];

/// other Caves you control plus Cave cards in your graveyard, three or more
fn three_caves(game: &Game, you: Seat, source: ObjId) -> bool {
    let on_battlefield = game
        .battlefield
        .iter()
        .filter(|p| **p != source && game.obj(**p).controller == you && game.has_subtype(**p, "Cave"))
        .count();
    let in_graveyard = game.player(you).graveyard.iter().filter(|c| game.has_subtype(**c, "Cave")).count();
    on_battlefield + in_graveyard >= 3
}

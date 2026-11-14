//! Chrome Replicator: "When Chrome Replicator enters, if you control two or more nonland, nontoken permanents with
//! the same name as one another, create a 4/4 colorless Construct artifact creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ChromeReplicator.ChromeReplicatorCondition", same_name)];

/// two of your nonland, nontoken permanents share a (nonempty) name
fn same_name(game: &Game, you: Seat, _: ObjId) -> bool {
    let names: Vec<&str> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let obj = game.obj(*o);
            obj.controller == you && !obj.token && !game.has_type(*o, CardType::Land)
        })
        .map(|o| game.obj(o).name())
        .filter(|n| !n.is_empty())
        .collect();
    names.iter().enumerate().any(|(i, n)| names[..i].contains(n))
}

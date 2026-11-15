//! Unforgiving One: "Whenever this creature attacks, return target creature card with mana value X or less from your
//! graveyard to the battlefield, where X is the number of modified creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("UnforgivingOne.UnforgivingOnePredicate", at_most_modified)];

/// `ModifiedPredicate`: a counter on it, or an Equipment, or an Aura its controller controls, attached
fn modified(game: &Game, id: ObjId) -> bool {
    let o = game.obj(id);
    o.counters.iter().any(|(_, n)| *n > 0)
        || game.attachments(id).any(|a| {
            game.has_subtype(a, "Equipment") || (game.has_subtype(a, "Aura") && game.obj(a).controller == o.controller)
        })
}

/// its mana value is at most the number of modified creatures you control
fn at_most_modified(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature) && modified(game, **o))
        .count() as u32;
    game.obj(id).def.mana_value() <= n
}

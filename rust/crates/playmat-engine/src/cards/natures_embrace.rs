//! Nature's Embrace: "Enchant creature or land. As long as enchanted permanent is a creature, it gets +2/+2. As long
//! as enchanted permanent is a land, it has "{T}: Add two mana of any one color.""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("NaturesEmbrace.NaturesEmbraceCondition.CREATURE", enchanted_creature),
    Handler::condition("NaturesEmbrace.NaturesEmbraceCondition.LAND", enchanted_land),
];

/// the source still on the battlefield, attached to a permanent of that card type
fn enchanted_is(game: &Game, source: ObjId, t: CardType) -> bool {
    game.obj(source).zone == Zone::Battlefield && game.attached(source).is_some_and(|to| game.has_type(to, t))
}

fn enchanted_creature(game: &Game, _you: Seat, source: ObjId) -> bool {
    enchanted_is(game, source, CardType::Creature)
}

fn enchanted_land(game: &Game, _you: Seat, source: ObjId) -> bool {
    enchanted_is(game, source, CardType::Land)
}

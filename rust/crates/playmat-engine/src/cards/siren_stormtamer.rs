//! Siren Stormtamer: "Flying. {U}, Sacrifice Siren Stormtamer: Counter target spell or ability that targets you or a
//! creature you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::target("SirenStormtamer.SirenStormtamerTarget", "new TargetStackObject()", targets_you_or_yours)];

/// a stack object with a target that's you, or a creature you control
fn targets_you_or_yours(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.stack.iter().filter(|i| i.card == id).any(|i| {
        i.targets.iter().flatten().any(|t| match t {
            Target::Player(p) => *p == you,
            Target::Obj(o) => {
                game.obj(*o).zone == Zone::Battlefield
                    && game.obj(*o).controller == you
                    && game.has_type(*o, CardType::Creature)
            }
        })
    })
}

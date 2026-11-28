//! Eriette of the Charmed Apple: "Each creature that's enchanted by an Aura you control can't attack you or
//! planeswalkers you control." (`CantAttackYouAllEffect(WhileOnBattlefield, filter, true)`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "ErietteOfTheCharmedApple.CantAttackYouAllEffect",
    applies,
    &[Restrict::CantAttack(cant)],
)
.args(&["while_on_battlefield", "one_of:filter", "one_of:true"])];

/// a creature an opponent controls (one of yours never attacks you), enchanted by an Aura you control
/// (`EnchantedBySourceControllerPredicate`)
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    game.has_type(permanent, CardType::Creature)
        && game.obj(permanent).controller != cx.controller
        && game.battlefield.iter().any(|a| {
            game.attached(*a) == Some(permanent)
                && game.obj(*a).controller == cx.controller
                && game.has_subtype(*a, "Aura")
        })
}

/// with two players, the only players and planeswalkers it could attack are yours
fn cant(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

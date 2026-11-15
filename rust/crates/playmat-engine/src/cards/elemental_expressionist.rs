//! Elemental Expressionist: "Magecraft — Whenever you cast or copy an instant or sorcery spell, choose target
//! creature you control. Until end of turn, it gains \"If this creature would leave the battlefield, exile it
//! instead of putting it anywhere else\" and \"When this creature is put into exile, create a 4/4 blue and red
//! Elemental creature token.\""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ElementalExpressionist.ElementalExpressionistTriggeredAbility",
    "new LeavesBattlefieldTriggeredAbility(new CreateTokenEffect(new Elemental44Token()))",
    check,
)];

/// `ZoneChangeTriggeredAbility(BATTLEFIELD, EXILED)`: it left the battlefield for exile
fn check(game: &Game, item: &StackItem) -> bool {
    game.obj(item.card).zone == Zone::Exile
}

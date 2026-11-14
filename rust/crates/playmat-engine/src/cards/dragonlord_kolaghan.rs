//! Dragonlord Kolaghan: "Whenever an opponent casts a creature or planeswalker spell with the same name as a card in
//! their graveyard, that player loses 10 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DragonlordKolaghan.DragonlordKolaghanTriggeredAbility",
    "new SpellCastOpponentTriggeredAbility(Zone.BATTLEFIELD, $0, StaticFilters.FILTER_SPELL, false, SetTargetPointer.PLAYER)",
    same_name,
)];

/// a creature or planeswalker spell, not face down, whose name a card in its caster's graveyard has
fn same_name(game: &Game, item: &StackItem) -> bool {
    let Some(s) = item.cast_spell else { return false };
    let o = game.obj(s);
    if o.face_down || !(game.has_type(s, CardType::Creature) || game.has_type(s, CardType::Planeswalker)) {
        return false;
    }
    let name = &o.def.name;
    game.player(o.controller).graveyard.iter().any(|c| game.obj(*c).def.name == *name)
}

//! `NoAbilityPredicate`: "with no abilities" (Fang-Druid Summoner, Rise from the Wreck: cards in a library or a
//! graveyard)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("NoAbilityPredicate", no_abilities)];

/// no ability but its spell ability (a vanilla card, or an instant or sorcery with only its spell); on the
/// battlefield, nothing granted either
fn no_abilities(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    o.def.abilities.iter().all(|a| matches!(a, AbilityDef::SpellAbility))
        && (o.zone != Zone::Battlefield || game.granted_abilities(id).is_empty())
}

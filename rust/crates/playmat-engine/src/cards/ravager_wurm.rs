//! Ravager Wurm: "Riot. When Ravager Wurm enters, choose up to one — • Ravager Wurm fights target creature you
//! don't control. • Destroy target land with an activated ability that isn't a mana ability."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("RavagerWurm.RavagerWurmPredicate", land_with_activated)];

/// a land with an activated ability that isn't a mana ability (`Ability::isNonManaActivatedAbility`: a special
/// action such as turning it face up isn't one), its own or granted
fn land_with_activated(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let nonmana = |a: &AbilityDef| a.activated().is_some() && !matches!(a, AbilityDef::TurnFaceUpAbility { .. });
    game.has_type(id, CardType::Land)
        && (game.obj(id).def.abilities.iter().any(nonmana) || game.granted_abilities(id).iter().any(nonmana))
}

//! `ReturnToBattlefieldUnderOwnerControlSourceEffect([tapped[, -1]])` and `(tapped, attacking, -1)`: "return it to
//! the battlefield [tapped] under its owner's control" (Tenacious Dead from its graveyard, Lamplight Phoenix from
//! exile after its cost), "tapped and attacking" (Warcry Phoenix)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnToBattlefieldUnderOwnerControlSourceEffect", apply)
    .args(&["one_of:true|false", "one_of:-1|true|false", "one_of:-1"])];

/// The source card from a graveyard or exile (any zone change count: the constructors written here take -1),
/// attacking only with the three-argument form's `attacking` (`Combat.addAttackingCreature`, nothing outside combat)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.source_id();
    if game.obj(o).token || !matches!(game.obj(o).zone, Zone::Graveyard | Zone::Exile) {
        return None;
    }
    // (two arguments are `(tapped, zoneChangeCounter)`, so the second is a flag only with three)
    if cx.args.len() == 2 && cx.args[1] != "-1" {
        return Some(());
    }
    let owner = game.obj(o).owner;
    enter_battlefield(game, d, o, owner, false);
    if cx.args.first().is_some_and(|a| a == "true") && game.obj(o).zone == Zone::Battlefield {
        game.obj_mut(o).tapped = true;
    }
    let attacking = cx.args.len() == 3 && cx.args[1] == "true";
    let in_combat = matches!(
        game.step,
        Some(Step::DeclareAttackers | Step::DeclareBlockers | Step::FirstCombatDamage | Step::CombatDamage)
    );
    if attacking && in_combat && game.obj(o).zone == Zone::Battlefield {
        let defender = game.opponent(owner);
        game.obj_mut(o).attacking = Some(defender);
        join_combat(game, o);
    }
    Some(())
}

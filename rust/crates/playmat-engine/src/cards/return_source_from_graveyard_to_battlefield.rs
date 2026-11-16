//! `ReturnSourceFromGraveyardToBattlefieldEffect(tapped, ownerControl[, haste, attacking])`, the two- and
//! four-argument forms (Persistent Marshstalker: "return this card from your graveyard to the battlefield tapped and
//! attacking"; Demonic Gifts's granted "return it to the battlefield under its owner's control")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnSourceFromGraveyardToBattlefieldEffect", apply)
    .args(&["bool", "bool", "one_of:true|false", "one_of:true|false"])];

/// while the source card is in a graveyard, it enters under you (its owner with `ownerControl`), tapped if so; with
/// `haste` it has haste for as long as it stays, with `attacking` it attacks (`Combat.addAttackingCreature`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.args.len() < 2 {
        return Some(());
    }
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let flag = |i: usize| cx.args.get(i).is_some_and(|a| a.trim() == "true");
    let to = if flag(1) { game.obj(card).owner } else { cx.controller() };
    let how = if flag(0) { crate::card::PutCards::BattlefieldTapped } else { crate::card::PutCards::Battlefield };
    if !put_cards(game, d, to, &[card], how) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    if flag(2) {
        let zcc = game.obj(card).zcc;
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: card,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
    }
    let in_combat = matches!(
        game.step,
        Some(Step::DeclareAttackers | Step::DeclareBlockers | Step::FirstCombatDamage | Step::CombatDamage)
    );
    if flag(3) && in_combat {
        let defender = game.opponent(to);
        let o = game.obj_mut(card);
        o.attacking = Some(defender);
        join_combat(game, card);
    }
    Some(())
}

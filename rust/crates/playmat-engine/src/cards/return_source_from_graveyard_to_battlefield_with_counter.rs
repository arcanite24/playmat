//! `ReturnSourceFromGraveyardToBattlefieldWithCounterEffect` (Gallia, Tragic Host; the finality-counter returns):
//! "return this card from your graveyard to the battlefield [tapped] with a <counter> on it".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnSourceFromGraveyardToBattlefieldWithCounterEffect", apply)];

/// `(counter, tapped[, ownerControl, haste, attacking])`: while the source card is in a graveyard, it enters under
/// you (its owner with `ownerControl`), tapped if so, with the counter (`AddCounterSourceReplacementEffect`, put by
/// you); with `haste` it has haste for as long as it stays, with `attacking` it attacks
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let flag = |i: usize| cx.args.get(i).is_some_and(|a| a.trim() == "true");
    let (name, n) = counter(cx.args.first().map(String::as_str).unwrap_or(""));
    let to = if flag(2) { game.obj(card).owner } else { cx.controller() };
    let how = if flag(1) { crate::card::PutCards::BattlefieldTapped } else { crate::card::PutCards::Battlefield };
    if !put_cards(game, d, to, &[card], how) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    if n > 0 {
        put_counters(game, card, &name, n, cx.controller());
    }
    if flag(3) {
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
    if flag(4) && in_combat {
        let defender = game.opponent(to);
        let o = game.obj_mut(card);
        o.attacking = Some(defender);
        join_combat(game, card);
    }
    Some(())
}

/// `CounterType.P1P1.createInstance(2)` as the engine's counter name and amount
fn counter(arg: &str) -> (String, i32) {
    let arg = arg.trim();
    let Some(rest) = arg.strip_prefix("CounterType.") else { return (String::new(), 0) };
    let Some((kind, n)) = rest.split_once(".createInstance(") else { return (String::new(), 0) };
    let n = n.trim_end_matches(')').trim().parse().unwrap_or(1);
    let name = match kind {
        "P1P1" => "+1/+1".to_string(),
        "M1M1" => "-1/-1".to_string(),
        k => k.to_lowercase().replace('_', " "),
    };
    (name, n)
}

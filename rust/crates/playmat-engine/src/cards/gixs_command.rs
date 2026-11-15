//! Gix's Command: "Choose two — • Put two +1/+1 counters on up to one creature. It gains lifelink until end of turn.
//! • Destroy each creature with power 2 or less. • Return up to two creature cards from your graveyard to your hand.
//! • Each opponent sacrifices a creature with the highest power among creatures they control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GixsCommand.GixsCommandCounterEffect", counters)
        .effects(&["new GainAbilityTargetEffect(LifelinkAbility.getInstance())"]),
    Handler::effect("GixsCommand.GixsCommandReturnEffect", back),
];

/// `chooseTarget(TargetCreaturePermanent(0, 1).withNotTarget(true))` (nothing asked with no creature): two +1/+1
/// counters on it, then lifelink until end of turn for it
fn counters(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    if creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, you, &creatures, &creatures, 0, 1, &rule)?.first() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    add_counters_by(game, o, "+1/+1", 2, Some(you));
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
    (!d.stopped()).then_some(())
}

/// `chooseTarget(TargetCardInYourGraveyard(0, 2, creature cards))` (nothing asked with none there), then those cards
/// into your hand together
fn back(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let graveyard = game.player(you).graveyard.clone();
    let creatures: Vec<ObjId> = graveyard.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    if creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &graveyard, &creatures, 0, 2, &rule)?;
    if chosen.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Hand) {
        return None;
    }
    Some(())
}

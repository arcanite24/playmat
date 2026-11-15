//! Terminal Velocity: "You may put an artifact or creature card from your hand onto the battlefield. That permanent
//! gains haste, "When this permanent leaves the battlefield, it deals damage equal to its mana value to each
//! creature," and "At the beginning of your end step, sacrifice this permanent.""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TerminalVelocity.TerminalVelocityEffect", apply)
        .effects(&[
            "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.Custom)",
            r#"new GainAbilityTargetEffect(new LeavesBattlefieldTriggeredAbility(new TerminalVelocityDamageEffect()).setTriggerPhrase("When this permanent leaves the battlefield, "), Duration.Custom)"#,
            r#"new GainAbilityTargetEffect(new BeginningOfEndStepTriggeredAbility(new SacrificeSourceEffect().setText("sacrifice this permanent")), Duration.Custom)"#,
        ])
        .filters(&["StaticFilters.FILTER_CARD_ARTIFACT_OR_CREATURE"]),
    Handler::effect("TerminalVelocity.TerminalVelocityDamageEffect", damage),
];

/// Up to one artifact or creature card from your hand chosen (`TargetCardInHand(0, 1, …)`), put onto the
/// battlefield; the permanent it became gains the three abilities for as long as it stays
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand: Vec<ObjId> = game.player(you).hand.clone();
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &possible, 0, 1, &rule)?;
    let Some(card) = chosen.first().copied() else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    for e in cx.effects {
        cx.run_on(game, d, std::slice::from_ref(e), vec![Target::Obj(card)]);
    }
    Some(())
}

/// The permanent as it last was (`getSourcePermanentOrLKI`): with a mana value of 1 or more, that much damage to
/// each creature, in battlefield order
fn damage(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    let mv = game.obj(me).def.mana_value() as i32;
    if mv < 1 {
        return Some(());
    }
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    for c in creatures {
        deal_damage(game, me, Target::Obj(c), mv, false);
    }
    Some(())
}

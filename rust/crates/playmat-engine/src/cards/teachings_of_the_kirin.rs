//! Teachings of the Kirin // Kirin-Touched Orochi: "Whenever Kirin-Touched Orochi attacks, choose one — • Exile
//! target creature card from a graveyard. When you do, create a 1/1 colorless Spirit creature token. • Exile target
//! noncreature card from a graveyard. When you do, put a +1/+1 counter on target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TeachingsOfTheKirin.KirinTouchedOrochiTokenEffect", token)
        .effects(&["new CreateTokenEffect(new SpiritToken())"]),
    Handler::effect("TeachingsOfTheKirin.KirinTouchedOrochiCounterEffect", counter)
        .effects(&["new AddCountersTargetEffect(CounterType.P1P1.createInstance())"])
        .filters(&["StaticFilters.FILTER_CONTROLLED_CREATURE"]),
];

/// The target card while it's still in a graveyard, exiled (`moveCards`); moved, a reflexive trigger waits
fn exiled(game: &mut Game, cx: &EffectCx) -> bool {
    let Some(card) = cx.obj(0) else { return false };
    if game.obj(card).zone != Zone::Graveyard {
        return false;
    }
    game.move_to(card, Zone::Exile);
    game.obj(card).zone == Zone::Exile
}

fn reflexive(game: &mut Game, cx: &EffectCx, rule: &str, effects: Vec<EffectDef>, targets: Vec<TargetDef>) {
    let delayed = Delayed {
        rule: rule.into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects,
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets,
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
}

/// `ReflexiveTriggeredAbility(new CreateTokenEffect(new SpiritToken()), false)`: no text of its own, so its rule is
/// the effect's text (`TriggeredAbilityImpl.getRule`)
fn token(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if exiled(game, cx) {
        reflexive(game, cx, "create a 1/1 colorless Spirit creature token.", cx.effects.to_vec(), Vec::new());
    }
    Some(())
}

/// the same with a +1/+1 counter on target creature you control (`TargetControlledCreaturePermanent`)
fn counter(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if exiled(game, cx) {
        let target = TargetDef::TargetPermanent {
            filter: cx.filters[0].clone(),
            min: 1,
            max: 1,
            distribute: None,
            x_count: false,
            count: None,
        };
        reflexive(game, cx, "put a +1/+1 counter on target creature you control.", cx.effects.to_vec(), vec![target]);
    }
    Some(())
}

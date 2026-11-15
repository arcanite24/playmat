//! Ascendant Spirit: "{S}{S}{S}: If Ascendant Spirit is a Warrior, put a flying counter on it and it becomes a Spirit
//! Warrior Angel with base power and toughness 4/4." and "{S}{S}{S}{S}: If Ascendant Spirit is an Angel, put two +1/+1
//! counters on it and it gains \"Whenever this creature deals combat damage to a player, draw a card.\""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AscendantSpirit.AscendantSpiritWarriorEffect", warrior).effects(&[
        "new AddCountersSourceEffect(CounterType.FLYING.createInstance())",
        "new AddCardSubTypeSourceEffect(Duration.Custom, SubType.SPIRIT, SubType.WARRIOR, SubType.ANGEL)",
        "new SetBasePowerToughnessSourceEffect(4, 4, Duration.Custom)",
    ]),
    Handler::effect("AscendantSpirit.AscendantSpiritAngelEffect", angel).effects(&[
        "new AddCountersSourceEffect(CounterType.P1P1.createInstance(2))",
        "new GainAbilitySourceEffect(new DealsCombatDamageToAPlayerTriggeredAbility(new DrawCardSourceControllerEffect(1), false), Duration.Custom)",
    ]),
];

/// if the source is still on the battlefield and a Warrior: the flying counter, then the subtypes and base 4/4
fn warrior(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) && game.has_subtype(cx.source_id(), "Warrior") {
        cx.run(game, d, &cx.effects[..3]);
    }
    Some(())
}

/// if the source is still on the battlefield and an Angel: two +1/+1 counters, then the trigger until it leaves
fn angel(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) && game.has_subtype(cx.source_id(), "Angel") {
        cx.run(game, d, &cx.effects[..2]);
    }
    Some(())
}

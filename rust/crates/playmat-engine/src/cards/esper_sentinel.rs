//! Esper Sentinel: "Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player
//! pays {X}, where X is Esper Sentinel's power."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "EsperSentinel.EsperSentinelTriggeredAbility",
        "new SpellCastOpponentTriggeredAbility(Zone.BATTLEFIELD, new EsperSentinelEffect(), StaticFilters.FILTER_SPELL_A_NON_CREATURE, false, SetTargetPointer.PLAYER)",
        first_noncreature,
    ),
    Handler::effect("EsperSentinel.EsperSentinelEffect", apply),
];

/// triggers only on that opponent's first noncreature spell this turn (not an intervening-if, so resolution skips check)
fn first_noncreature(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    let Some(Target::Player(opponent)) = item.targets.first().and_then(|ts| ts.first()) else {
        return false;
    };
    game.player(*opponent).noncreature_cast == 1
}

/// opponent may pay {X} where X is Esper Sentinel's power (or LKI power); unless paid, controller draws a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let power = if cx.source_on_battlefield(game) {
        game.power_toughness(cx.source_id()).0
    } else {
        cx.item.lki_power.unwrap_or_else(|| game.obj(cx.source_id()).def.power)
    }
    .max(0);
    let cost = ManaCost { generic: power as u32, ..ManaCost::default() };
    if choose_use(game, d, opponent, &format!("Pay {{{power}}}?"))? {
        let paid = pay_mana(game, d, opponent, &cost);
        if d.stopped() {
            return None;
        }
        if paid {
            return Some(());
        }
    }
    draw(game, you, 1);
    Some(())
}

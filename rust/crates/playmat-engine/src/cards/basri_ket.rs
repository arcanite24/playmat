//! Basri Ket: "−2: Whenever one or more nontoken creatures attack this turn, create that many 1/1 white Soldier
//! creature tokens that are tapped and attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "BasriKet.BasriKetTriggeredAbility",
        "new AttacksWithCreaturesTriggeredAbility(new BasriKetSoldiersEffect(), 1, StaticFilters.FILTER_CREATURE_NON_TOKEN)",
        any_batch,
    ),
    // the effect the trigger's `checkTrigger` sets: `CreateTokenEffect(new SoldierToken(), attackingNonTokens, true,
    // true)`
    Handler::effect("BasriKet.BasriKetSoldiersEffect", soldiers).tokens(&["SoldierToken"]),
];

/// As many tapped and attacking Soldiers as there are nontoken creatures attacking
fn soldiers(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.attacking.is_some() && !ob.token && game.has_type(**o, CardType::Creature)
        })
        .count() as i32;
    if n == 0 {
        return Some(());
    }
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(n),
        tapped_and_attacking: true,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}

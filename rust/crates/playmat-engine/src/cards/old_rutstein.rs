//! Old Rutstein: "When Old Rutstein enters or at the beginning of your upkeep, mill a card. If a land card is milled
//! this way, create a Treasure token. If a creature card is milled this way, create a 1/1 green Insect creature
//! token. If a noncreature, nonland card is milled this way, create a Blood token."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "OldRutstein.OldRutsteinTriggeredAbility",
        "new OrTriggeredAbility(Zone.BATTLEFIELD, new OldRutsteinEffect(), new EntersBattlefieldTriggeredAbility(null), new BeginningOfUpkeepTriggeredAbility(null))",
        always,
    ),
    Handler::effect("OldRutstein.OldRutsteinEffect", apply).tokens(&["TreasureToken", "InsectToken", "BloodToken"]),
];

fn always(_game: &Game, _t: &StackItem) -> bool {
    true
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(1).copied().collect();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(1) }]);
    let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let land = milled.iter().any(|c| game.obj(*c).is(CardType::Land));
    let creature = milled.iter().any(|c| game.obj(*c).is(CardType::Creature));
    let other = milled.iter().any(|c| !game.obj(*c).is(CardType::Land) && !game.obj(*c).is(CardType::Creature));
    for (made, i) in [(land, 0), (creature, 1), (other, 2)] {
        if made {
            cx.create_tokens(game, d, &cx.tokens[i], 1);
        }
    }
    Some(())
}

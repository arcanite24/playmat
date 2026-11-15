//! Death's Oasis: "Whenever a nontoken creature you control dies, mill two cards. Then return a creature card with
//! lesser mana value than the creature that died from your graveyard to your hand. {1}, Sacrifice Death's Oasis: You
//! gain life equal to the greatest mana value among creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "DeathsOasis.DeathsOasisTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new DeathsOasisEffect(), false, StaticFilters.FILTER_CONTROLLED_CREATURE_NON_TOKEN)",
        any_batch,
    ),
    // the trigger's two effects (`MillCardsControllerEffect(2)`, then the return), as one the base trigger can name
    Handler::effect("DeathsOasis.DeathsOasisEffect", apply).effects(&["new MillCardsControllerEffect(2)"]),
];

/// Mill two; then, with a creature card in your graveyard of lesser mana value than the creature that died (as it
/// last was), you choose one (`TargetCardInYourGraveyard`, not targeted, `Player.choose`) and it returns to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let o = game.obj(died);
    let mv = o.lki_def.as_ref().unwrap_or(&o.def).mana_value();
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    let fits: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.is(CardType::Creature) && game.obj(*c).def.mana_value() < mv)
        .collect();
    if fits.is_empty() {
        return Some(());
    }
    if let Some(&c) = choose_cards_rounds(game, d, you, &fits, 1, 1)?.first() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}

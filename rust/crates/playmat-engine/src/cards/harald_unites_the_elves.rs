//! Harald Unites the Elves: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I — Mill three cards. You may put an Elf or Tyvar card from your graveyard onto the battlefield.
//! II — Put a +1/+1 counter on each Elf you control.
//! III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("HaraldUnitesTheElves.HaraldUnitesTheElvesEffect", mill_and_return),
    Handler::trigger(
        "HaraldUnitesTheElves.HaraldUnitesTheElvesTriggeredAbility",
        "new AttacksAllTriggeredAbility(new BoostTargetEffect(-1, -1), false, new FilterControlledPermanent(SubType.ELF), SetTargetPointer.NONE, false)",
        elf_attacks,
    ),
];

/// Mill three, then up to one Elf or Tyvar card from your graveyard (`TargetCardInYourGraveyard(0, 1, …, true)`,
/// `Player.choose`, asked even with nothing to choose) onto the battlefield
fn mill_and_return(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: Amount::Fixed(3) }]);
    let graveyard = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> =
        graveyard.iter().copied().filter(|c| game.has_subtype(*c, "Elf") || game.has_subtype(*c, "Tyvar")).collect();
    // (`Player.choose(Target)`: asked even with no Elf or Tyvar card there, "done" the answer)
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &graveyard, &possible, 0, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}

/// an Elf you control attacking: the base trigger's filter is all of it
fn elf_attacks(_: &Game, _: &StackItem) -> bool {
    true
}

//! The Bloodsky Massacre: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I — Create a 2/3 red Demon Berserker creature token with menace. II — Whenever a Berserker attacks this turn, you
//! draw a card and you lose 1 life. III — Add {R} for each Berserker you control. Until end of turn, you don't lose
//! this mana as steps and phases end."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TheBloodskyMassacre.TheBloodskyMassacreAbility",
        r#"new AttacksAllTriggeredAbility(new DrawCardSourceControllerEffect(1), false, new FilterCreaturePermanent(SubType.BERSERKER, "a Berserker"), SetTargetPointer.NONE, false)"#,
        berserker,
    ),
    Handler::effect("TheBloodskyMassacre.TheBloodskyMassacreEffect", apply),
];

/// a Berserker attacking: the base trigger is all of it
fn berserker(_: &Game, _: &StackItem) -> bool {
    true
}

/// {R} for each Berserker you control, kept until end of turn (`addMana(…, emptyOnTurnsEnd: true)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Berserker"))
        .count() as u32;
    let p = game.player_mut(you);
    p.mana_pool.colored[Color::R.index()] += n;
    p.eot_mana.colored[Color::R.index()] += n;
    Some(())
}

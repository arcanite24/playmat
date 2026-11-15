//! War-Trained Slasher: "Menace. Whenever War-Trained Slasher attacks a battle, double its power until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "WarTrainedSlasher.WarTrainedSlasherTriggeredAbility",
        "new AttacksTriggeredAbility(new WarTrainedSlasherEffect())",
        attacks_battle,
    ),
    Handler::effect("WarTrainedSlasher.WarTrainedSlasherEffect", apply),
];

/// `DECLARED_ATTACKERS` with this attacking (the base trigger), its defender a battle (`getDefenderId`)
fn attacks_battle(game: &Game, t: &StackItem) -> bool {
    game.obj(t.card).attacking_pw.is_some_and(|(b, _)| game.has_type(b, CardType::Battle))
}

/// +X/+0 until end of turn, X its power now (none for 0), while it's still the permanent that attacked
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let power = game.power_toughness(cx.source_id()).0;
    if power != 0 {
        boost(game, cx.source_id(), power, 0, Duration::EndOfTurn);
    }
    Some(())
}

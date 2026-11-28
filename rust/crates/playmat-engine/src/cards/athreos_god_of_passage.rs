//! Athreos, God of Passage: "Whenever another creature you own dies, return it to your hand unless target opponent
//! pays 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AthreosGodOfPassage.AthreosDiesCreatureTriggeredAbility",
        "new DiesCreatureTriggeredAbility($0, $1, $2)",
        any,
    ),
    Handler::effect("AthreosGodOfPassage.AthreosGodOfPassageReturnEffect", apply),
];

fn any(_: &Game, _: &StackItem) -> bool {
    true
}

/// With 3 life to pay, the opponent is asked; paid, nothing more. Otherwise the card, still in the graveyard, goes
/// to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some((card, _)) = cx.item.died else { return Some(()) };
    if let Some(opp) = cx.player(0) {
        if game.player(opp).life >= 3 {
            let message = format!(
                "Pay 3 life to prevent that {} returns to {}'s hand?",
                log::object(game, card),
                log::player(you)
            );
            if choose_use(game, d, opp, &message)? {
                lose_life(game, opp, 3);
                return Some(());
            }
        }
    }
    if game.obj(card).zone == Zone::Graveyard {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

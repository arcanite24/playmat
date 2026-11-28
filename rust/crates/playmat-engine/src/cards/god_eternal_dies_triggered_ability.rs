//! `GodEternalDiesTriggeredAbility`: "When this dies or is put into exile from the battlefield, you may put it into
//! its owner's library third from the top." (Ilharg, the Raze-Boar; God-Eternal Oketra)

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "GodEternalDiesTriggeredAbility",
        "new LeavesBattlefieldTriggeredAbility(new GodEternalEffect(), true)",
        to_graveyard_or_exile,
    ),
    Handler::effect("GodEternalEffect", third_from_top),
];

/// it left the battlefield for a graveyard or exile (`checkTrigger`, only as it triggers: resolving, it holds)
fn to_graveyard_or_exile(game: &Game, t: &StackItem) -> bool {
    game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability)
        || matches!(game.obj(t.card).zone, Zone::Graveyard | Zone::Exile)
}

/// `putCardsOnTopXOfLibrary(card, 3)`: third from the top of its owner's library (on the bottom when the library has
/// fewer than two cards)
fn third_from_top(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if !matches!(game.obj(card).zone, Zone::Graveyard | Zone::Exile) {
        return Some(());
    }
    let owner = game.obj(card).owner;
    let size = game.player(owner).library.len();
    game.move_to(card, Zone::Library);
    if game.obj(card).zone == Zone::Library && !game.obj(card).token {
        let lib = &mut game.player_mut(owner).library;
        lib.retain(|x| *x != card);
        if size + 1 < 3 {
            lib.push(card);
        } else {
            lib.insert(2.min(lib.len()), card);
        }
    }
    Some(())
}

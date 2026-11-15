//! Black Widow, Super Spy: "Whenever Black Widow deals combat damage to a player, that player exiles cards from the
//! top of their library until they exile a nonland card. You may put a +1/+1 counter on Black Widow. If you don't,
//! you may cast the exiled nonland card until end of turn and mana of any type can be spent to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlackWidowSuperSpy.BlackWidowSuperSpyEffect", apply)];

/// One card at a time into exile until a nonland one; with Black Widow still on the battlefield, "Put a +1/+1
/// counter on Black Widow?"; no (or gone): the nonland card may be cast this turn with mana of any type
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(damaged) = cx.player(0) else { return Some(()) };
    let mut to_cast = None;
    while let Some(card) = game.player(damaged).library.first().copied() {
        game.move_to(card, Zone::Exile);
        if !game.has_type(card, CardType::Land) {
            to_cast = Some(card);
            break;
        }
    }
    if cx.source_on_battlefield(game) && choose_use(game, d, you, "Put a +1/+1 counter on Black Widow?")? {
        add_counters_by(game, cx.source_id(), "+1/+1", 1, Some(you));
        return Some(());
    }
    if let Some(card) = to_cast {
        if game.obj(card).zone == Zone::Exile {
            let turn = game.turn;
            let o = game.obj_mut(card);
            o.may_play = Some((you, turn));
            o.cast_only = true;
            o.any_mana = true;
        }
    }
    Some(())
}

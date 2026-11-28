//! Thor, God of Thunder: "When Thor enters, exile target Equipment, instant, or sorcery card from your graveyard.
//! Until the end of your next turn, you may play that card. Whenever you cast a noncreature spell, Thor deals damage
//! equal to that spell's mana value to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ThorGodOfThunder.ThorGodOfThunderEffect", exile),
    Handler::value("ThorGodOfThunder.ThorGodOfThunderValue", spell_mana_value),
];

/// `makeCardPlayable(…, Duration.UntilEndOfYourNextTurn)`, as `ExileTopXMayPlayUntilEffect` keeps it
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    game.obj_mut(card).exiled_with = Some(cx.source);
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    game.obj_mut(card).may_play = Some((you, until));
    Some(())
}

/// a cast trigger's X is the spell's mana value (`getValue("spellCast")`)
fn spell_mana_value(_game: &Game, _you: Seat, _source: ObjId, x: i32) -> i32 {
    x
}

//! Golos, Tireless Pilgrim: "When Golos enters, you may search your library for a land card, put that card onto the
//! battlefield tapped, then shuffle. {2}{W}{U}{B}{R}{G}: Exile the top three cards of your library. You may play them
//! this turn without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GolosTirelessPilgrim.GolosTirelessPilgrimEffect", apply)];

/// `exileAndPlayFromExile(top three, YOU, Duration.EndOfTurn, withoutMana, !anyColor, !onlyCast)`: the cards
/// exiled together, then those in exile may be played by you this turn without paying their mana costs
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let turn = game.turn;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.play_free = true;
        }
    }
    Some(())
}

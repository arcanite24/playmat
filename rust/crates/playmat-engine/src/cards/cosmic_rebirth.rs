//! Cosmic Rebirth: "Choose target permanent card in your graveyard. If it has mana value 3 or less, you may put it
//! onto the battlefield. If you don't put it onto the battlefield, put it into your hand. You gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CosmicRebirth.CosmicRebirthEffect", apply)];

/// Mana value 3 or less: "Put it onto the battlefield or your hand?" (yes: the battlefield); else the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.obj(0)?;
    let battlefield = game.obj(card).def.mana_value() <= 3 && {
        let q = Query::YesNo { message: "Put it onto the battlefield or your hand?".into() };
        loop {
            match ask(game, d, you, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return None,
                _ => {}
            }
        }
    };
    if battlefield {
        enter_battlefield(game, d, card, you, false);
    } else {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

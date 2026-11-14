//! Aberrant Mind Sorcerer: "Psionic Spells — When this creature enters, choose target instant or sorcery card in
//! your graveyard, then roll a d20. 1-9 | You may put that card on top of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AberrantMindSorcerer.AberrantMindSorcererEffect", apply)];

/// the card still in the graveyard, then the controller's yes
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let message = format!("Put {} on top of your library?", game.obj(c).def.name);
    if choose_use(game, d, cx.controller(), &message)? {
        to_library(game, c, true);
    }
    Some(())
}

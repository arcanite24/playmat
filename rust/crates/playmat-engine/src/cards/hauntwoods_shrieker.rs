//! Hauntwoods Shrieker: "{1}{G}: Reveal target face-down permanent. If it's a creature card, you may turn it face
//! up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HauntwoodsShrieker.HauntwoodsShriekerEffect", apply)];

/// The permanent revealed; its card a creature card, you're asked, and a yes turns it face up
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(o, Zone::Battlefield)] });
    let card = game.obj(o).face_up.clone().unwrap_or_else(|| game.obj(o).def.clone());
    if card.types.contains(&CardType::Creature)
        && game.obj(o).face_down
        && choose_use(game, d, cx.controller(), &format!("Turn {} face up?", card.name))?
    {
        turn_face_up(game, o, false);
    }
    Some(())
}

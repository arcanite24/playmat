//! Deceiver of Form: "At the beginning of combat on your turn, reveal the top card of your library. If a creature card
//! is revealed this way, you may have creatures you control other than Deceiver of Form become copies of that card
//! until end of turn. You may put that card on the bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeceiverOfForm.DeceiverOfFormEffect", apply)];

/// The top card revealed; a creature card, "Let creatures you control other than … becomes copies of … until end of
/// turn?" and each of them a copy of it (`CopyEffect(EndOfTurn, new PermanentCard(card))`); then "Move … to the
/// bottom of your library?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let Some(&top) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(src).zcc;
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: vec![(top, Zone::Library)] });
    if game.has_type(top, CardType::Creature) {
        let message = format!(
            "Let creatures you control other than {} becomes copies of {} until end of turn?",
            log::object(game, src),
            log::object(game, top)
        );
        if choose_use(game, d, you, &message)? {
            let copy = game.obj(top).def.clone();
            let creatures: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| *o != src && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
                .collect();
            for c in creatures {
                become_copy_until_end_of_turn(game, c, copy.clone());
            }
        }
    }
    let message = format!("Move {} to the bottom of your library?", log::object(game, top));
    if choose_use(game, d, you, &message)? && game.obj(top).zone == Zone::Library {
        let lib = &mut game.player_mut(you).library;
        lib.retain(|x| *x != top);
        lib.push(top);
    }
    Some(())
}

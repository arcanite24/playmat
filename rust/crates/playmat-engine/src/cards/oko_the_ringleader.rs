//! Oko, the Ringleader: "At the beginning of combat on your turn, Oko becomes a copy of up to one target creature
//! you control until end of turn, except he has hexproof. +1: Draw two cards. If you've committed a crime this turn,
//! discard a card. Otherwise, discard two cards. −1: Create a 3/3 green Elk creature token. −5: For each other
//! nonland permanent you control, create a token that's a copy of that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("OkoTheRingleader.OkoTheRingleaderCopySelfEffect", copy_self),
    Handler::effect("OkoTheRingleader.OkoTheRingleaderCopyTokenEffect", copy_tokens),
];

/// Oko (still that permanent) becomes a copy of the target until end of turn, the copy with hexproof
fn copy_self(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    if !cx.source_on_battlefield(game) || game.obj(creature).zone != Zone::Battlefield {
        return None;
    }
    let mut def = (*game.obj(creature).copiable()).clone();
    def.abilities.push(AbilityDef::HexproofAbility);
    become_copy_until_end_of_turn(game, cx.source_id(), Arc::new(def));
    Some(())
}

/// A token copy of each other nonland permanent you control, in battlefield order
fn copy_tokens(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != src && game.obj(*o).controller == you && !game.has_type(*o, CardType::Land))
        .collect();
    for o in theirs {
        token_copy(game, d, o, you, None);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

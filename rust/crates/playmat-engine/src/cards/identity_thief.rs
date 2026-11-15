//! Identity Thief: "Whenever Identity Thief attacks, you may exile another target nontoken creature. If you do,
//! Identity Thief becomes a copy of that creature until end of turn. Return the exiled card to the battlefield under
//! its owner's control at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IdentityThief.IdentityThiefEffect", apply)];

/// The target exiled; the delayed return of that card; then the Thief (if it's still that permanent) becomes a copy
/// of the creature as it last was on the battlefield until end of turn
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let saved = game.obj(o).copiable();
    game.move_to(o, Zone::Exile);
    let fixed: Vec<(ObjId, u32)> = if game.obj(o).zone == Zone::Exile && !game.obj(o).token {
        vec![(o, game.obj(o).zcc)]
    } else {
        Vec::new()
    };
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return the exiled card to the battlefield under its owner's \
               control at the beginning of the next end step."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToBattlefieldTargetEffect { your_control: false, tapped: false }],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    if cx.source_on_battlefield(game) {
        become_copy_until_end_of_turn(game, cx.source_id(), saved);
    }
    Some(())
}

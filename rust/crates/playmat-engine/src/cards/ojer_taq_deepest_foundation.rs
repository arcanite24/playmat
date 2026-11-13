//! Ojer Taq, Deepest Foundation: "When Ojer Taq dies, return it to the battlefield tapped and transformed under its
//! owner's control." Temple of Civilization: "{2}{W}, {T}: Transform Temple of Civilization. Activate only if you
//! attacked with three or more creatures this turn and only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("OjerTaqDeepestFoundation.OjerTaqDeepestFoundationTransformEffect", apply),
    Handler::condition("OjerTaqDeepestFoundation.TempleOfCivilizationCondition", attacked_with_three),
];

/// `PlayerAttackedWatcher.getNumberOfAttackersCurrentTurn`: three or more attackers you declared this turn
fn attacked_with_three(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).attackers_this_turn >= 3
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    return_transformed(game, d, cx, true)
}

/// `getSourceCardIfItStillExists`: the card still in the graveyard it died to; it enters with its back face up
/// (`VALUE_KEY_ENTER_TRANSFORMED`), under the ability's controller (`controller.moveCards`)
pub(super) fn return_transformed(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, tapped: bool) -> Option<()> {
    let (card, zcc) = cx.source;
    if game.obj(card).zone != Zone::Graveyard || game.obj(card).zcc != zcc {
        return Some(());
    }
    let Some(back) = game.obj(card).def.transform_back.clone() else { return Some(()) };
    let o = game.obj_mut(card);
    o.front = Some(std::mem::replace(&mut o.def, back));
    let how = if tapped { crate::card::PutCards::BattlefieldTapped } else { crate::card::PutCards::Battlefield };
    if !put_cards(game, d, cx.controller(), &[card], how) {
        return None;
    }
    Some(())
}

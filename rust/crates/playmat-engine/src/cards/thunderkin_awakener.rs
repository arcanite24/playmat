//! Thunderkin Awakener: "Haste. Whenever Thunderkin Awakener attacks, choose target Elemental creature card in your
//! graveyard with toughness less than Thunderkin Awakener's toughness. Return that card to the battlefield tapped
//! and attacking. Sacrifice it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("ThunderkinAwakener.ThunderkinAwakenerPredicate", lesser_toughness),
    Handler::effect("ThunderkinAwakener.ThunderkinAwakenerEffect", apply),
];

/// the card's toughness is less than the source's (`getSourcePermanentOrLKI`)
fn lesser_toughness(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let s = game.obj(src);
    let theirs = if s.zone == Zone::Battlefield {
        game.power_toughness(src).1
    } else {
        match s.toughness_lki {
            Some(t) => t,
            None => return false,
        }
    };
    game.obj(id).def.toughness < theirs
}

/// `ReturnFromGraveyardToBattlefieldTargetEffect(tapped, attacking)`; the permanent it became is sacrificed at the
/// beginning of the next end step ("Sacrifice <name>")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: true, owner: false, counters: None, attacking: true };
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(card).zcc;
    let delayed = crate::state::Delayed {
        rule: format!("At the beginning of the next end step, Sacrifice {}.", game.obj(card).name()),
        sacrifice: vec![(card, zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

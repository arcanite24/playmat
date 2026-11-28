//! Avatar Destiny: "When enchanted creature dies, mill cards equal to its power. Return this card to its owner's
//! hand and up to one creature card milled this way to the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AvatarDestiny.AvatarDestinyEffect", apply)];

/// The creature's power as it last was (`getValue("attachedTo")`), that many milled, the Aura back to its owner's
/// hand from the zone it went to (`ReturnToHandSourceEffect(false, true)`), then up to one creature card of the
/// milled ones chosen (`Player.choose`, asked whatever was milled) and put onto the battlefield tapped: Java's
/// `moveCards(card, BATTLEFIELD, source, game, true, …)` passes `tapped` true
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // (not always Java's: a Gnarlid Colony that died 3/3 with this Aura, blocking with Monastery Swiftspear, milled
    // 2 in Java, 3 here: wave 25a round 1 game 9)
    let count = died_enchanted(game, cx).map_or(0, |o| game.obj(o).power_lki.unwrap_or_else(|| game.obj(o).def.power));
    let top: Vec<ObjId> = game.player(you).library.iter().take(count.max(0) as usize).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    cx.run(game, d, &[EffectDef::ReturnToHandSourceEffect { battlefield_only: false }]);
    let creatures: Vec<ObjId> = milled.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &milled, &creatures, 0, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}

/// The object that left the battlefield with this Aura among its attachments (Unholy Indenture's), the nearest the
/// top of its graveyard when there are several
fn died_enchanted(game: &Game, cx: &EffectCx) -> Option<ObjId> {
    let (aura, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    (0..game.objs.len() as u32)
        .map(ObjId)
        .filter(|o| {
            let c = game.obj(*o);
            c.zone != Zone::Battlefield
                && c.lki_attachments.iter().any(|(a, z)| *a == aura && (*z == zcc || *z + 1 == zcc))
        })
        .min_by_key(|o| {
            let c = game.obj(*o);
            let yard = &game.player(c.owner).graveyard;
            let from_top = yard.iter().rev().position(|g| g == o).unwrap_or(if c.token { 0 } else { usize::MAX });
            (from_top, std::cmp::Reverse(o.0))
        })
}

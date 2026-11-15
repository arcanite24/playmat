//! Unholy Indenture: "Enchant creature. When enchanted creature dies, return that card to the battlefield under your
//! control with a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnholyIndenture.UnholyIndentureReturnEffect", apply)];

/// The creature that died (`getValue("attachedTo")`: the trigger names no card, so it's the object that left the
/// battlefield with this Aura among its attachments, the last to leave), not a token and still the card that went
/// to the graveyard, put onto the battlefield under your control with a +1/+1 counter as it enters
/// (`setEnterWithCounters`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = died_enchanted(game, cx) else { return Some(()) };
    if game.obj(card).token || game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: false, counters: None, attacking: false };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 1));
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    Some(())
}

/// The object that left the battlefield with this Aura (as it was there: it may have gone to the graveyard since)
/// among its attachments, the nearest the top of its graveyard when there are several
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

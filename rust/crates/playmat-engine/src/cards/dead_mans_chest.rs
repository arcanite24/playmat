//! Dead Man's Chest: "Enchant creature an opponent controls. When enchanted creature dies, exile cards equal to its
//! power from the top of its owner's library. You may cast spells from among those cards for as long as they remain
//! exiled, and you may spend mana as though it were mana of any type to cast those spells."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeadMansChest.DeadMansChestEffect", apply)];

/// The creature's power as it last was (`getValue("attachedTo")`): that many cards from the top of its owner's
/// library exiled together; the nonland ones castable by you while they stay exiled, with mana of any type
/// (`PlayFromNotOwnHandZoneTargetEffect`, `DeadMansChestSpendManaEffect`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(died) = died_enchanted(game, cx) else { return Some(()) };
    let amount = game.obj(died).power_lki.unwrap_or_else(|| game.obj(died).def.power);
    if amount <= 0 {
        return Some(());
    }
    let owner = game.obj(died).owner;
    let top: Vec<ObjId> = game.player(owner).library.iter().take(amount as usize).copied().collect();
    let src = cx.source;
    for c in top {
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone != Zone::Exile {
            continue;
        }
        game.obj_mut(c).exiled_with = Some(src);
        if game.obj(c).def.is(CardType::Land) {
            continue;
        }
        let o = game.obj_mut(c);
        o.may_play = Some((you, u32::MAX));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.any_mana = true;
        o.cast_only = true;
    }
    Some(())
}

/// The creature that died enchanted by the Chest (`getValue("attachedTo")`): the object that left the battlefield
/// with the Chest among its attachments, the one that left last when there are several (Mask of Griselbrand's)
fn died_enchanted(game: &Game, cx: &EffectCx) -> Option<ObjId> {
    // (the Chest as it was when the trigger fired: by now it's in its graveyard too)
    let (chest, zcc) = cx.source;
    (0..game.objs.len() as u32)
        .map(ObjId)
        .filter(|o| {
            let c = game.obj(*o);
            c.zone != Zone::Battlefield && c.lki_attachments.contains(&(chest, zcc))
        })
        .min_by_key(|o| {
            let c = game.obj(*o);
            let yard = &game.player(c.owner).graveyard;
            let from_top = yard.iter().rev().position(|g| g == o).unwrap_or(if c.token { 0 } else { usize::MAX });
            (from_top, std::cmp::Reverse(o.0))
        })
}

//! Zack Fair: "{1}, Sacrifice Zack Fair: Target creature you control gains indestructible until end of turn. Put
//! Zack Fair's counters on that creature and attach an Equipment that was attached to Zack Fair to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZackFair.ZackFairEffect", apply)];

/// The Equipment attached to Zack as it last was (`getSourcePermanentOrLKI().getAttachments()`) that are still on
/// the battlefield: none, nothing; one, that one; more, the controller chooses one (`TargetPermanent`, not
/// targeted). It's attached to the target creature.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(creature) = cx.obj(0) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let src = cx.source_id();
    let attached: Vec<(ObjId, u32)> = if cx.source_on_battlefield(game) {
        game.attachments(src).map(|a| (a, game.obj(a).zcc)).collect()
    } else {
        game.obj(src).lki_attachments.clone()
    };
    let equipment: Vec<ObjId> = attached
        .into_iter()
        .filter(|(a, z)| {
            let o = game.obj(*a);
            o.zone == Zone::Battlefield && o.zcc == *z && game.has_subtype(*a, "Equipment")
        })
        .map(|(a, _)| a)
        .collect();
    let chosen = match equipment.len() {
        0 => return Some(()),
        1 => equipment[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, cx.controller(), &equipment, &equipment, 1, 1, &rule)?.first() {
                Some(e) => *e,
                None => return Some(()),
            }
        }
    };
    let zcc = game.obj(creature).zcc;
    game.attach_to(chosen, creature, zcc);
    Some(())
}

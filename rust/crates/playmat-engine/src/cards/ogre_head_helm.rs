//! Ogre-Head Helm: "Equipped creature gets +2/+2. Whenever Ogre-Head Helm or equipped creature deals combat damage
//! to a player, you may sacrifice it. If you do, discard your hand, then draw three cards. Reconfigure {3}"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "OgreHeadHelm.OgreHeadHelmTriggeredAbility",
        "new DealsDamageToAPlayerAllTriggeredAbility(new OgreHeadHelmEffect(), StaticFilters.FILTER_PERMANENT, false, SetTargetPointer.PERMANENT, true)",
        check,
    ),
    Handler::effect("OgreHeadHelm.OgreHeadHelmEffect", apply),
];

/// combat damage to a player by the Helm itself or by the creature it's attached to (as it is, or as it last was);
/// the pointer at what dealt it
fn check(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Obj(by)) = t.targets.first().and_then(|g| g.first()).copied() else { return false };
    if by == t.card {
        return true;
    }
    let helm = game.obj(t.card);
    let attached = if helm.zone == Zone::Battlefield { helm.attached_to } else { helm.lki_attached_to };
    attached.is_some_and(|(a, _)| a == by)
}

/// That permanent still on the battlefield: "Sacrifice <name>?"; sacrificed, the hand discarded, then three cards
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(it) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let name = game.obj(it).def.name.clone();
    if !choose_use(game, d, you, &format!("Sacrifice {name}?"))? {
        return Some(());
    }
    sacrifice(game, it);
    if game.obj(it).zone == Zone::Battlefield {
        return Some(());
    }
    let hand = game.player(you).hand.clone();
    for c in &hand {
        discard_card(game, you, *c);
    }
    game.discarded_cards();
    draw(game, you, 3);
    Some(())
}

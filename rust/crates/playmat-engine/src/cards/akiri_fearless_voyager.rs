//! Akiri, Fearless Voyager: "Whenever you attack a player with one or more equipped creatures, draw a card. {W}: You
//! may unattach an Equipment from a creature you control. If you do, tap that creature and it gains indestructible
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AkiriFearlessVoyager.AkiriFearlessVoyagerTriggeredAbility",
        "new AttacksPlayerWithCreaturesTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE, SetTargetPointer.NONE)",
        equipped_attacker,
    ),
    Handler::effect("AkiriFearlessVoyager.AkiriFearlessVoyagerEffect", unattach)
        .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"]),
];

/// `DEFENDER_ATTACKED` by you: a player attacked by a creature with an Equipment attached (checked as it triggers
/// only: unattached by then, it still draws)
fn equipped_attacker(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    game.active == Some(item.controller)
        && game.battlefield.iter().any(|a| {
            let o = game.obj(*a);
            o.controller == item.controller && o.attacking.is_some() && o.attacking_pw.is_none() && equipped(game, *a)
        })
}

fn equipped(game: &Game, creature: ObjId) -> bool {
    game.battlefield.iter().any(|e| {
        let o = game.obj(*e);
        o.attached_to.is_some_and(|(to, zcc)| to == creature && zcc == game.obj(creature).zcc)
            && game.has_subtype(*e, "Equipment")
    })
}

/// An Equipment attached to a creature you control (`TargetPermanent(1, 1, filter, true)`): nothing to choose or no
/// to "Unnattach an equipment from a creature you control?" does nothing; else the chosen one comes off, and the
/// creature taps and gains indestructible until end of turn
fn unattach(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|e| {
            game.has_subtype(*e, "Equipment")
                && game.obj(*e).attached_to.is_some_and(|(to, zcc)| {
                    let c = game.obj(to);
                    c.zone == Zone::Battlefield
                        && c.zcc == zcc
                        && c.controller == you
                        && game.has_type(to, CardType::Creature)
                })
        })
        .collect();
    if possible.is_empty() || !choose_use(game, d, you, "Unnattach an equipment from a creature you control?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?;
    let Some(&equipment) = chosen.first() else { return Some(()) };
    let Some((creature, _)) = game.obj(equipment).attached_to else { return Some(()) };
    game.obj_mut(equipment).attached_to = None;
    tap(game, creature);
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(creature)]);
    Some(())
}

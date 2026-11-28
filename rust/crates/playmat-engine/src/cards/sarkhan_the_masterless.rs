//! Sarkhan the Masterless: "Whenever a creature attacks you or a planeswalker you control, each Dragon you control
//! deals 1 damage to that creature. +1: Until end of turn, each planeswalker you control becomes a 4/4 red Dragon
//! creature and gains flying."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SarkhanTheMasterless.SarkhanTheMasterlessDamageEffect", dragons_deal_damage),
    Handler::effect("SarkhanTheMasterless.SarkhanTheMasterlessBecomeDragonEffect", become_dragons).effects(&[
        "new BecomesCreatureTargetEffect(new CreatureToken(4,4,\"4/4 red Dragon creature\",SubType.DRAGON).withColor(\"R\").withAbility(FlyingAbility.getInstance()), false, false, Duration.EndOfTurn)",
    ]),
];

/// The attacker (the target pointer), if it's still on the battlefield: each Dragon you control, in battlefield
/// order, deals it 1 damage, one event each (`creature.damage(1, dragon, …)`)
fn dragons_deal_damage(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(creature) = cx.obj(0) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let dragons: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.obj(*p).controller == you && game.has_subtype(*p, "Dragon"))
        .collect();
    for dragon in dragons {
        if game.obj(creature).zone != Zone::Battlefield {
            break;
        }
        if game.obj(dragon).zone == Zone::Battlefield {
            deal_damage(game, dragon, Target::Obj(creature), 1, false);
        }
    }
    Some(())
}

/// The planeswalkers you control as it resolves (`init`'s `affectedObjectList`) each become a 4/4 red Dragon creature
/// with flying until end of turn, in place of their types and subtypes (layers 4, 5, 6 and 7b)
fn become_dragons(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let walkers: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.obj(*p).controller == you && game.has_type(*p, CardType::Planeswalker))
        .map(Target::Obj)
        .collect();
    if !walkers.is_empty() {
        cx.run_on(game, d, &cx.effects[..1], walkers);
    }
    Some(())
}

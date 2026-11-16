//! Siege Dragon: "Flying. When Siege Dragon enters, destroy all Walls your opponents control. Whenever Siege Dragon
//! attacks, if defending player controls no Walls, it deals 2 damage to each creature without flying that player
//! controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "SiegeDragon.SiegeDragonAttacksTriggeredAbility",
        "new AttacksTriggeredAbility(new SiegeDragonDamageEffect(), false)",
        no_walls,
    ),
    Handler::effect("SiegeDragon.SiegeDragonDamageEffect", apply),
];

/// the player the attacking Siege Dragon's defending player (`getDefendingPlayerId`: none once it left combat)
fn defending(game: &Game, src: ObjId) -> Option<Seat> {
    let o = game.obj(src);
    if o.zone != Zone::Battlefield {
        return None;
    }
    o.attacking
}

/// the intervening "if": there's a defending player, and no Wall creature on the battlefield is theirs
fn no_walls(game: &Game, t: &StackItem) -> bool {
    let Some(p) = defending(game, t.card) else { return false };
    !game.battlefield.iter().any(|o| {
        game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && game.has_subtype(*o, "Wall")
    })
}

/// 2 damage to each creature without flying the defending player controls, in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(p) = defending(game, src) else { return Some(()) };
    let hit: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == p
                && game.has_type(*o, CardType::Creature)
                && !game.has_ability(*o, &AbilityDef::FlyingAbility)
        })
        .collect();
    for c in hit {
        if game.obj(c).zone == Zone::Battlefield {
            deal_damage(game, src, Target::Obj(c), 2, false);
        }
    }
    Some(())
}

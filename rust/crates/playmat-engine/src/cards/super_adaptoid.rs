//! Super-Adaptoid: "Super-Adaptoid's power is equal to the number of legendary creatures you control. Whenever
//! Super-Adaptoid enters or attacks, choose another target creature. If that creature has haste and Super-Adaptoid
//! doesn't, put a haste counter on Super-Adaptoid. Do the same for flying, first strike, double strike, deathtouch,
//! indestructible, lifelink, menace, reach, trample, and vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("SuperAdaptoid.LegendaryCreaturesYouControlCount", legendary_creatures),
    Handler::effect("SuperAdaptoid.SuperAdaptoidEffect", apply),
];

/// the legendary creatures you control
fn legendary_creatures(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == you
                && game.has_type(**o, CardType::Creature)
                && game.obj(**o).def.supertypes.iter().any(|s| s == "LEGENDARY")
        })
        .count() as i32
}

/// `COUNTER_MAPPINGS`, in Java's order
const MAPPINGS: [(AbilityDef, &str); 11] = [
    (AbilityDef::HasteAbility, "haste"),
    (AbilityDef::FlyingAbility, "flying"),
    (AbilityDef::FirstStrikeAbility, "first strike"),
    (AbilityDef::DoubleStrikeAbility, "double strike"),
    (AbilityDef::DeathtouchAbility, "deathtouch"),
    (AbilityDef::IndestructibleAbility, "indestructible"),
    (AbilityDef::LifelinkAbility, "lifelink"),
    (AbilityDef::MenaceAbility, "menace"),
    (AbilityDef::ReachAbility, "reach"),
    (AbilityDef::TrampleAbility, "trample"),
    (AbilityDef::VigilanceAbility, "vigilance"),
];

/// The target and Super-Adaptoid both on the battlefield (`game.getPermanent` of each): a counter of each kind
/// whose ability the target has and Super-Adaptoid doesn't, one kind at a time
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    let me = cx.source_id();
    if game.obj(t).zone != Zone::Battlefield || game.obj(me).zone != Zone::Battlefield {
        return Some(());
    }
    for (ability, counter) in &MAPPINGS {
        if game.has_ability(t, ability) && !game.has_ability(me, ability) {
            add_counters_by(game, me, counter, 1, Some(cx.controller()));
        }
    }
    Some(())
}

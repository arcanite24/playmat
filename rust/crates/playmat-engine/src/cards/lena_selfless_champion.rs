//! Lena, Selfless Champion: "When Lena enters, create a 1/1 white Soldier creature token for each nontoken creature
//! you control. Sacrifice Lena: Creatures you control with power less than Lena's power gain indestructible until end
//! of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LenaSelflessChampion.LenaSelflessChampionEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"])];

/// Lena's power as it last was; the creatures you control with less power now (`GainAbilityControlledEffect` until
/// end of turn: the ones affected as it resolves) gain indestructible
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let power = match cx.item.lki_power {
        Some(p) => p,
        None if cx.source_on_battlefield(game) => game.power_toughness(cx.source_id()).0,
        None => return Some(()),
    };
    let affected: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
                && game.power_toughness(*o).0 < power
        })
        .collect();
    for o in affected {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
    }
    Some(())
}

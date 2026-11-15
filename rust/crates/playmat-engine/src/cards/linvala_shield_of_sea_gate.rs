//! Linvala, Shield of Sea Gate: "Flying. At the beginning of combat on your turn, if you have a full party, choose
//! target nonland permanent an opponent controls. Until your next turn, it can't attack or block, and its activated
//! abilities can't be activated. Sacrifice Linvala: Choose hexproof or indestructible. Creatures you control gain that
//! ability until end of turn."

use super::*;

const CANT_ATTACK_OR_BLOCK: &str = "LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateCantAttackOrBlock";
const CANT_ACTIVATE: &str = "LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateCantActivate";

pub const HANDLERS: &[Handler] = &[
    // the `RestrictionEffect` until your next turn: its `canAttack`/`canBlock` and its `canUseActivatedAbilities`
    Handler::effect("LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateRestrictionEffect", restrict),
    Handler::restriction(
        "LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateCantAttackOrBlock",
        the_target,
        &[Restrict::CantAttack(never_attack), Restrict::CantBlock(never_block)],
    )
    .until_your_next_turn(),
    Handler::rule("LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateCantActivate", Rule::CantActivate(cant_activate))
        .until_your_next_turn(),
    Handler::effect("LinvalaShieldOfSeaGate.LinvalaShieldOfSeaGateEffect", choose).effects(&[
        "new GainAbilityControlledEffect(HexproofAbility.getInstance(), Duration.EndOfTurn, StaticFilters.FILTER_CONTROLLED_CREATURE)",
        "new GainAbilityControlledEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn, StaticFilters.FILTER_CONTROLLED_CREATURE)",
    ]),
];

/// the restriction and the can't-activate rule, both pointing at the trigger's target, until your next turn
fn restrict(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut effects = Vec::new();
    for name in [CANT_ATTACK_OR_BLOCK, CANT_ACTIVATE] {
        let handler = serde_json::from_value(serde_json::json!(name)).ok()?;
        effects.push(EffectDef::CardStatic {
            handler,
            args: Vec::new(),
            effects: Vec::new(),
            filters: Vec::new(),
            abilities: Vec::new(),
        });
    }
    cx.run(game, d, &effects);
    Some(())
}

/// the target, while it's still that permanent
fn the_target(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.target_obj(game, 0) == Some(id)
}

fn never_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn never_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}

/// the target's activated abilities (its mana abilities aren't stopped here, as Deadlock Trap's)
fn cant_activate(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: usize) -> bool {
    cx.target_obj(game, 0) == Some(id)
}

/// "Choose hexproof or indestructible" (yes: Hexproof), then creatures you control gain it until end of turn
fn choose(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let hexproof = choose_use(game, d, cx.controller(), "Choose hexproof or indestructible")?;
    let e = if hexproof { &cx.effects[0] } else { &cx.effects[1] };
    cx.run(game, d, std::slice::from_ref(e));
    Some(())
}

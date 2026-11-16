//! Majestic Myriarch: "Majestic Myriarch's power and toughness are each equal to twice the number of creatures you
//! control. At the beginning of each combat, if you control a creature with flying, Majestic Myriarch gains flying
//! until end of turn. The same is true for first strike, double strike, deathtouch, haste, hexproof,
//! indestructible, lifelink, menace, reach, trample, and vigilance."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("MajesticMyriarch.MajesticMyriarchEffect", apply)
    .filters(&[
        "filterFlying",
        "filterFirstStrike",
        "filterDoubleStrike",
        "filterDeathtouch",
        "filterHaste",
        "filterIndestructible",
        "filterLifelink",
        "filterMenace",
        "filterReach",
        "filterTrample",
        "filterVigilance",
    ])
    .effects(&[
        "new GainAbilitySourceEffect(FlyingAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(FirstStrikeAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(DoubleStrikeAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(DeathtouchAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(HexproofAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(LifelinkAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(new MenaceAbility(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(ReachAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(TrampleAbility.getInstance(), Duration.EndOfTurn)",
        "new GainAbilitySourceEffect(VigilanceAbility.getInstance(), Duration.EndOfTurn)",
    ])];

/// In Java's order, each keyword's check (`containsControlled(filter, source, game, 1)`: a creature you control
/// with it, the source included) and, if it holds, its `GainAbilitySourceEffect` until end of turn. Hexproof is
/// `HexproofBaseAbility`: hexproof or any "hexproof from".
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let mut filters = cx.filters.iter();
    for (k, effect) in cx.effects.iter().enumerate() {
        let mine: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
        let holds = if k == HEXPROOF {
            mine.iter().any(|o| hexproof_base(game, *o))
        } else {
            let Some(filter) = filters.next() else { return Some(()) };
            mine.iter().any(|o| game.filter_matches(filter, you, Some(src), *o))
        };
        if holds {
            cx.run(game, d, std::slice::from_ref(effect));
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}

/// the place of hexproof among the effects (it has no filter: `AbilityPredicate(HexproofBaseAbility.class)`)
const HEXPROOF: usize = 5;

/// a creature with hexproof or a "hexproof from" ability
fn hexproof_base(game: &Game, o: ObjId) -> bool {
    if !game.has_type(o, CardType::Creature) {
        return false;
    }
    if game.has_ability(o, &AbilityDef::HexproofAbility) {
        return true;
    }
    let mut all: Vec<AbilityDef> = game.obj(o).def.abilities.clone();
    all.extend(game.granted_abilities(o));
    all.iter().any(|a| matches!(a, AbilityDef::HexproofFromAbility { .. }) && game.has_ability(o, a))
}

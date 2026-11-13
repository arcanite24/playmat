//! Goblin Negotiation: "Goblin Negotiation deals X damage to target creature. Create a number of 1/1 red Goblin
//! creature tokens equal to the amount of excess damage dealt to that creature this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GoblinNegotiation.GoblinNegotiationEffect", apply)
    .effects(&["new CreateTokenEffect(new GoblinToken())"])];

/// X damage with excess (`damageWithExcess`), then that many Goblins
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let excess = damage_with_excess(game, cx.source_id(), o, cx.item.x.max(0));
    if excess > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = crate::card::Amount::Fixed(excess);
        }
        cx.run_on(game, d, &[e], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

/// `Permanent.damageWithExcess`: only lethal damage is dealt (`getLethalDamage`: toughness less damage, 1 from a
/// deathtouch source, a planeswalker's loyalty); what's beyond it is returned
pub(super) fn damage_with_excess(game: &mut Game, source: ObjId, o: ObjId, amount: i32) -> i32 {
    let mut lethal = i32::MAX;
    if game.has_type(o, CardType::Creature) {
        lethal = (game.power_toughness(o).1 - game.obj(o).damage).max(0);
        if game.has_ability(source, &AbilityDef::DeathtouchAbility) {
            lethal = lethal.min(1);
        }
    }
    if game.has_type(o, CardType::Planeswalker) {
        lethal = lethal.min(game.obj(o).counter("loyalty"));
    }
    if game.has_type(o, CardType::Battle) {
        lethal = lethal.min(game.obj(o).counter("defense"));
    }
    let excess = (amount - lethal).max(0);
    deal_damage(game, source, Target::Obj(o), amount.min(lethal), false);
    excess
}

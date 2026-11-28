//! Kinzu of the Bleak Coven: "Flying. Whenever another nontoken creature you control dies, you may pay 2 life and
//! exile it. If you do, create a token that's a copy of that creature, except it's 1/1 and has toxic 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KinzuOfTheBleakCoven.KinzuOfTheBleakCovenEffect", apply)];

/// When 2 life can be paid: "Pay 2 Life and Exile <card>?", the life paid, the card exiled, and then a token copy of
/// the card (`PermanentCard(card)`) that's 1/1 and has toxic 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let cost = CostDef::PayLifeCost { amount: 2 };
    if !can_pay_cost(game, you, cx.source_id(), &cost) {
        return Some(());
    }
    let message = format!("Pay 2 Life and Exile {}?", game.obj(card).name());
    if !choose_use(game, d, you, &message)? || !pay_cost(game, d, you, cx.source_id(), &cost) {
        return Some(());
    }
    game.move_to(card, Zone::Exile);
    token_copy_modified(game, d, card, you, 1, &|def: &mut CardDef| {
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
        def.abilities.push(AbilityDef::ToxicAbility { amount: 1, rule: String::new() });
    });
    if d.stopped() {
        return None;
    }
    Some(())
}

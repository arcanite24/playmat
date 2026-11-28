//! Azor's Gateway // Sanctum of the Sun: "{1}, {T}: Draw a card, then exile a card from your hand. If cards with five
//! or more different mana values are exiled with Azor's Gateway, you gain 5 life, untap Azor's Gateway, and transform
//! it. // {T}: Add X mana of any one color, where X is your life total."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AzorsGateway.AzorsGatewayEffect", exile_from_hand),
    Handler::condition("AzorsGateway.AzorsGatewayCondition", five_mana_values),
];

/// a card from your hand (`TargetCardInHand`, `Player.choose`; the rule says "exile", so it isn't auto-chosen),
/// exiled with this
fn exile_from_hand(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.obj_mut(card).exiled_with = Some(cx.source);
    }
    Some(())
}

/// five or more different mana values among the cards in this permanent's exile zone
fn five_mana_values(game: &Game, _you: Seat, source: ObjId) -> bool {
    let zone = (source, game.obj(source).zcc);
    let mut values: Vec<u32> = Vec::new();
    for c in game.exile.iter().filter(|c| game.obj(**c).exiled_with == Some(zone)) {
        let mv = game.obj(*c).def.mana_value();
        if !values.contains(&mv) {
            values.push(mv);
        }
    }
    values.len() >= 5
}

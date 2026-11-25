//! Invasion of Tarkir: "When Invasion of Tarkir enters, reveal any number of Dragon cards from your hand. When you
//! do, Invasion of Tarkir deals X plus 2 damage to any other target, where X is the number of cards revealed this
//! way." Its back face, Defiant Thundermaw: "Whenever a Dragon you control attacks, it deals 2 damage to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("InvasionOfTarkir.InvasionOfTarkirEffect", reveal),
    Handler::effect("InvasionOfTarkir.DefiantThundermawEffect", thundermaw),
];

/// `player.choose(hand, TargetCardInHand(0, MAX, Dragon cards))` (nothing asked without one), the chosen revealed,
/// then the reflexive trigger, whatever was revealed: X plus 2 damage to any other target
fn reveal(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let dragons: Vec<ObjId> =
        game.player(you).hand.iter().copied().filter(|c| game.has_subtype(*c, "Dragon")).collect();
    let mut chosen = Vec::new();
    if !dragons.is_empty() {
        chosen = choose_cards(game, d, you, &dragons, &dragons, 0, dragons.len(), "")?;
    }
    if !chosen.is_empty() {
        let src = cx.source_id();
        let zcc = game.obj(src).zcc;
        let cards = chosen.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards });
    }
    let n = chosen.len() as i32 + 2;
    let e: EffectDef = serde_json::from_value(serde_json::json!({"class": "DamageTargetEffect", "amount": n})).ok()?;
    let target: TargetDef = serde_json::from_value(serde_json::json!({"class": "TargetAnyTarget", "another": true})).ok()?;
    let delayed = Delayed {
        rule: format!("{{this}} deals {n} damage to any other target."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// the attacking Dragon (`attackerRef`, the trigger's attacker) deals 2 damage to the target
fn thundermaw(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let attacker = cx.item.entering?;
    let target = cx.target(0)?;
    deal_damage(game, attacker, target, 2, false);
    Some(())
}

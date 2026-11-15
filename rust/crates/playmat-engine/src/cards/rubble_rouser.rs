//! Rubble Rouser: "When this creature enters, you may discard a card. If you do, draw a card. {T}, Exile a card from
//! your graveyard: Add {R}. When you do, this creature deals 1 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RubbleRouser.RubbleRouserDamageEffect", apply)
    .effects(&["new DamagePlayersEffect(1, TargetController.OPPONENT)"])];

/// `fireReflexiveTriggeredAbility`: "this creature deals 1 damage to each opponent", waiting to go on the stack
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let delayed = Delayed {
        rule: "This creature deals 1 damage to each opponent.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

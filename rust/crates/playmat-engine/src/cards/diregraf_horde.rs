//! Diregraf Horde: "When Diregraf Horde enters the battlefield, create two 2/2 black Zombie creature tokens with
//! decayed. When you do, exile up to two target cards from graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiregrafHorde.DiregrafHordeEffect", apply)
    .tokens(&["ZombieDecayedToken"])
    .effects(&["new ExileTargetEffect()"])];

/// the two Zombies, then `fireReflexiveTriggeredAbility`: up to two target cards from graveyards exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let target: TargetDef =
        serde_json::from_value(serde_json::json!({"class": "TargetCardInGraveyard", "min": 0, "max": 2})).ok()?;
    let delayed = Delayed {
        rule: "exile up to two target cards from graveyards".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

//! Obzedat, Ghost Council: "When Obzedat enters, target opponent loses 2 life and you gain 2 life. At the beginning
//! of your end step you may exile Obzedat. If you do, return it to the battlefield under its owner's control at the
//! beginning of your next upkeep. It gains haste."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ObzedatGhostCouncil.ObzedatGhostCouncilExileSourceEffect", apply),
    Handler::effect("ObzedatGhostCouncil.ObzedatGhostCouncilReturnEffect", back)
        .effects(&["new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.WhileOnBattlefield)"]),
];

/// Obzedat (`game.getPermanent(sourceId)`, whichever permanent it is now) into exile; there, a delayed trigger at the
/// beginning of your next upkeep holding the card
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(src, Zone::Exile);
    if game.obj(src).zone != Zone::Exile {
        return Some(());
    }
    let back: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": "ObzedatGhostCouncil.ObzedatGhostCouncilReturnEffect"}),
    )
    .ok()?;
    let delayed = Delayed {
        rule: "Return {this} to the battlefield under its owner's control at the beginning of your next upkeep. It gains \
               haste"
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: game.obj(src).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, cx.controller()) });
    Some(())
}

/// The card, from whatever zone it's in now, onto the battlefield under its owner's control; then haste for that
/// permanent while it's on the battlefield (`GainAbilitySourceEffect(WhileOnBattlefield)`)
fn back(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    let o = game.obj(card);
    if o.token || matches!(o.zone, Zone::Battlefield | Zone::Stack) {
        return Some(());
    }
    let owner = o.owner;
    enter_battlefield(game, d, card, owner, false);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
    (!d.stopped()).then_some(())
}

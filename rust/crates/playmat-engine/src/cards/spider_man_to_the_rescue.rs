//! Spider-Man, To the Rescue: "Flash. Vigilance. Reach. No One Dies! — When Spider-Man enters, you may tap him. When
//! you do, another target nonattacking creature you control gains indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpiderManToTheRescue.SpiderManToTheRescueEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance())"])
    .filters(&["filter"])];

/// Spider-Man still on the battlefield and untapped: "Tap <Spider-Man [id]>?"; tapped, `fireReflexiveTriggeredAbility`
/// (indestructible until end of turn for another target nonattacking creature you control)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    if !cx.source_on_battlefield(game) || game.obj(src).tapped {
        return Some(());
    }
    let r = crate::replay::reference(game, src);
    if !choose_use(game, d, you, &format!("Tap {} [{r}]?", game.obj(src).name()))? {
        return Some(());
    }
    tap(game, src);
    if !game.obj(src).tapped {
        return Some(());
    }
    let target =
        TargetDef::TargetPermanent { filter: cx.filters[0].clone(), min: 1, max: 1, distribute: None, x_count: false, count: None };
    let delayed = Delayed {
        rule: "Another target nonattacking creature you control gains indestructible until end of turn.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}

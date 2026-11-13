//! Ziatora, the Incinerator: "At the beginning of your end step, you may sacrifice another creature. When you do,
//! Ziatora deals damage equal to that creature's power to any target and you create three Treasure tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZiatoraTheIncinerator.ZiatoraTheIncineratorEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken(), 3)"])
    .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE"])];

/// `TargetSacrifice(0, 1, …)` chosen with `Player.choose`; then a reflexive trigger made with the sacrificed
/// creature's power as it last was (`new DamageTargetEffect(power)` and the Treasures, `TargetAnyTarget`), whose
/// text is Java's generated one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you && *o != cx.source_id())
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(o).0;
    sacrifice(game, o);
    let mut effects = vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(power) }];
    effects.extend(cx.effects.iter().cloned());
    let delayed = Delayed {
        rule: format!("{{this}} deals {power} damage to any target and create three Treasure tokens."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects,
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetAnyTarget {
            min: 1,
            max: 1,
            another: false,
            not_first: false,
            distribute: None,
            x_count: false,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

//! Tiger-Tribe Hunter: "Trample. Pack tactics — Whenever Tiger-Tribe Hunter attacks, if you attacked with creatures
//! with total power 6 or greater this combat, you may sacrifice another creature. When you do, Tiger-Tribe Hunter
//! deals damage equal to the sacrificed creature's power to target creature."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("TigerTribeHunter.TigerTribeHunterEffect", apply)
    .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE", "StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// `TargetPermanent(0, 1, …, true)` chosen with `Player.choose`; the chosen creature's power (at least 0) as it was,
/// its sacrifice, then a reflexive trigger dealing that much damage to target creature
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you && *o != cx.source_id())
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(o).0.max(0);
    sacrifice(game, o);
    if game.obj(o).zone == Zone::Battlefield {
        return Some(());
    }
    let delayed = Delayed {
        // (`getRule` adds a '.' after the text's own)
        rule: "{this} deals damage equal to the sacrificed creature's power to target creature..".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(power) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetPermanent {
            filter: cx.filters[1].clone(),
            min: 1,
            max: 1,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

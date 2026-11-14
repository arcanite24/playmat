//! Surtland Flinger: "Whenever Surtland Flinger attacks, you may sacrifice another creature. When you do, Surtland
//! Flinger deals damage equal to the sacrificed creature's power to any target. If the sacrificed creature was a
//! Giant, Surtland Flinger deals twice that much damage instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SurtlandFlinger.SurtlandFlingerEffect", apply)
    .effects(&["new DamageTargetEffect(1)"])];

/// `TargetPermanent(0, 1, another creature you control, notTarget)` with `Player.choose` (asked even with nothing to
/// choose); its power (doubled for a Giant, at least 0) as it was, the sacrifice, then the reflexive trigger
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != me && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let rule = item_rule(game, cx.item);
    let &o = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?.first()?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let mut power = game.power_toughness(o).0;
    if game.has_subtype(o, "Giant") {
        power *= 2;
    }
    let power = power.max(0);
    // the reflexive trigger goes after what the sacrifice fires at once and ahead of what its zone change fires (the
    // sacrificed creature's dies trigger)
    let outer = game.sac_insert.replace(game.pending.len());
    sacrifice(game, o);
    let at = game.sac_insert.unwrap_or(game.pending.len()).min(game.pending.len());
    game.sac_insert = outer;
    let mut e = cx.effects[0].clone();
    if let EffectDef::DamageTargetEffect { amount } = &mut e {
        *amount = crate::card::Amount::Fixed(power);
    }
    let target = TargetDef::TargetAnyTarget {
        min: 1,
        max: 1,
        another: false,
        not_first: false,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "{this} deals damage equal to the sacrificed creature's power to any target. If the sacrificed creature \
               was a Giant, {this} deals twice that much damage instead.."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.insert(at, StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(me, you) });
    Some(())
}

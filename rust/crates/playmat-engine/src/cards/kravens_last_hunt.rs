//! Kraven's Last Hunt: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I — Mill five cards. When you do, this Saga deals damage equal to the greatest power among creature cards in your
//! graveyard to target creature. II — Target creature you control gets +2/+2 until end of turn. III — Return target
//! creature card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("KravensLastHunt.KravensLastHuntValue", greatest_power),
    Handler::effect("KravensLastHunt.KravensLastHuntEffect", apply)
        .effects(&["new DamageTargetEffect(KravensLastHuntValue.instance)"])
        .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"]),
];

/// the greatest power among creature cards in your graveyard (0 without any)
fn greatest_power(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).is(CardType::Creature))
        .map(|c| game.obj(*c).def.power)
        .max()
        .unwrap_or(0)
}

/// `fireReflexiveTriggeredAbility`, always: the damage to target creature waits to go on the stack
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "{this} deals damage equal to the greatest power among creature cards in your graveyard to target \
               creature."
            .into(),
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

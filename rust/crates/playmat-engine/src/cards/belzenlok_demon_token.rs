//! Rite of Belzenlok's Demon token: "At the beginning of your upkeep, sacrifice another creature. If you can't, this
//! creature deals 6 damage to you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BelzenlokDemonToken.BelzenlokDemonTokenEffect", apply)
    .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE"])
    .effects(&["new SacrificeControllerEffect(StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE,1,\"\")"])];

/// another creature you control to sacrifice when there is one (`PermanentsOnBattlefieldCount`), else 6 damage to
/// you from this creature
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !matching(game, &cx.filters[0], you, cx.source_id()).is_empty() {
        cx.run(game, d, &cx.effects[..1]);
    } else {
        deal_damage(game, cx.source_id(), Target::Player(you), 6, false);
    }
    Some(())
}

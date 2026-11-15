//! Twilight Diviner: "Whenever one or more other creatures you control enter, if they entered or were cast from a
//! graveyard, create a token that's a copy of one of them. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TwilightDiviner.TwilightDivinerTriggeredAbility",
        "new ZoneChangeBatchTriggeredAbility(new TwilightDivinerEffect(), null, Zone.BATTLEFIELD, StaticFilters.FILTER_OTHER_CONTROLLED_CREATURE, TargetController.YOU, false).setTriggersLimitEachTurn(1)",
        from_graveyard,
    ),
    Handler::effect("TwilightDiviner.TwilightDivinerEffect", apply),
];

/// `checkEvent`'s zones: from a graveyard, or a spell cast from one
fn came_from_graveyard(game: &Game, o: ObjId, from: Zone) -> bool {
    from == Zone::Graveyard || (from == Zone::Stack && game.obj(o).cast_from == Some(Zone::Graveyard))
}

fn from_graveyard(game: &Game, t: &StackItem) -> bool {
    t.batch.iter().any(|(o, _, from)| came_from_graveyard(game, *o, *from))
}

/// a token copy of one of them: the only one, or the one its controller chooses ("permanent to copy")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let them: Vec<ObjId> = cx
        .item
        .batch
        .iter()
        .filter(|(o, zcc, from)| {
            came_from_graveyard(game, *o, *from) && game.obj(*o).zone == Zone::Battlefield && game.obj(*o).zcc == *zcc
        })
        .map(|(o, _, _)| *o)
        .collect();
    let of = match them.len() {
        0 => return Some(()),
        1 => them[0],
        _ => *choose_cards(game, d, you, &them, &them, 1, 1, "permanent to copy")?.first()?,
    };
    token_copy(game, d, of, you, None);
    Some(())
}

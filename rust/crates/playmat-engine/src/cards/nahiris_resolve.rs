//! Nahiri's Resolve: "Creatures you control get +1/+0 and have haste. At the beginning of your end step, exile any
//! number of nontoken artifacts and/or creatures you control. Return those cards to the battlefield under their
//! owner's control at the beginning of your next upkeep."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[Handler::effect("NahirisResolve.NahirisResolveExileEffect", apply)];

/// `player.choose` of a not-targeted `TargetPermanent(0, MAX, nontoken artifacts and/or creatures you control)` (one
/// round, asked even with none), those exiled into the source's zone, then
/// `AtTheBeginOfYourNextUpkeepDelayedTriggeredAbility(NahirisResolveReturnEffect)` on them (`FixedTargets` of the
/// cards in exile): they return together under their owners' control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let ob = game.obj(*o);
            ob.controller == you
                && !ob.token
                && (game.has_type(*o, CardType::Artifact) || game.has_type(*o, CardType::Creature))
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards_with(game, d, you, &possible, &possible, 0, usize::MAX, &rule, false)?;
    for c in &chosen {
        if game.obj(*c).zone == Zone::Battlefield {
            game.move_to(*c, Zone::Exile);
            if game.obj(*c).zone == Zone::Exile {
                game.obj_mut(*c).exiled_with = Some(cx.source);
            }
        }
    }
    let fixed: Vec<(ObjId, u32)> =
        chosen.iter().filter(|c| game.obj(**c).zone == Zone::Exile).map(|c| (*c, game.obj(*c).zcc)).collect();
    let delayed = Delayed {
        rule: "At the beginning of your next upkeep, return those cards to the battlefield under their owner's control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToBattlefieldTargetEffect { your_control: false, tapped: false }],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

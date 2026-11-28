//! Kiora, Master of the Depths: "+1: Untap up to one target creature and up to one target land. −2: Reveal the top
//! four cards of your library. You may put a creature card and/or a land card from among them into your hand. Put the
//! rest into your graveyard. −8: You get an emblem with "Whenever a creature you control enters, you may have it
//! fight target creature." Then create three 8/8 blue Octopus creature tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KioraMasterOfTheDepths.KioraUntapEffect", untap),
    Handler::effect("KioraMasterOfTheDepths.KioraRevealEffect", super::in_the_presence_of_ages::apply),
    Handler::effect("KioraMasterOfTheDepthsEmblem.KioraFightEffect", fight),
];

/// every target of both groups that's still a permanent untaps, in order
fn untap(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..cx.legal.len() {
        for o in cx.objs(i) {
            if game.obj(o).zone == Zone::Battlefield {
                untap_permanent(game, o);
            }
        }
    }
    Some(())
}

/// the creature that entered (the trigger's pointer) fights the target creature (`Permanent.fight`, as
/// `FightTargetsEffect` has the first target fight the second), both still creatures
fn fight(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let entered = cx.item.entering?;
    let target = cx.obj(0)?;
    let groups = [vec![Target::Obj(entered)], vec![Target::Obj(target)]];
    super::apply_each_effect(game, d, cx.item, &[EffectDef::FightTargetsEffect], &groups, cx.source);
    Some(())
}

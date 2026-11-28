//! Archfiend of the Dross: "Whenever a creature an opponent controls dies, its controller loses 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchfiendOfTheDross.ArchfiendOfTheDrossEffect", apply)];

/// `getValue("creatureDied")`'s controller (as it died) loses 2 life
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (_, controller) = cx.item.died?;
    lose_life(game, controller, 2);
    Some(())
}

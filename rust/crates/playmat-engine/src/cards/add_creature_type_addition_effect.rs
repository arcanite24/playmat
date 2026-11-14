//! `AddCreatureTypeAdditionEffect(subtype, false)` after a return to the battlefield: "it's a Phyrexian in addition
//! to its other types" (Portal to Phyrexia, Terror of Towashi). The target that came back has the subtype for as long
//! as it stays on the battlefield (`Duration.Custom`, discarded once it's gone).

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AddCreatureTypeAdditionEffect", apply).args(&["one_of:SubType.PHYREXIAN", "one_of:false"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let subtype = "Phyrexian".to_string();
    let o = cx.obj(0)?;
    if game.obj(o).zone == Zone::Battlefield {
        let zcc = game.obj(o).zcc;
        game.subtypes_while_there.push((o, zcc, subtype));
    }
    Some(())
}

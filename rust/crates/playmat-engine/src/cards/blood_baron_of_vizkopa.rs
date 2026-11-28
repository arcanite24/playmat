//! Blood Baron of Vizkopa: "Lifelink, protection from white and from black. As long as you have 30 or more life and
//! an opponent has 10 or less life, Blood Baron of Vizkopa gets +6/+6 and has flying."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "BloodBaronOfVizkopa.BloodBaronOfVizkopaEffect",
    itself_while_ahead,
    Layer::ModifyPt(plus_six),
)
.and_layers(&[Layer::Ability(flying)])];

/// the Baron, while you have 30 or more life and an opponent still in the game has 10 or less
fn itself_while_ahead(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
        && game.player(cx.controller).life >= 30
        && (0..game.seats()).any(|p| p != cx.controller && !game.player(p).lost && game.player(p).life <= 10)
}

fn plus_six(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (6, 6)
}

fn flying(_: &Game, _: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    vec![AbilityDef::FlyingAbility]
}

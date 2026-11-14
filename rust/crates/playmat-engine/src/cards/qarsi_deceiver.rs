//! Qarsi Deceiver: "{T}: Add {C}. Spend this mana only to cast a face-down creature spell, pay a mana cost to turn a
//! manifested creature face up, or pay a morph cost."

use super::*;
use crate::card::{AbilityDef, FaceDown};

pub const HANDLERS: &[Handler] = &[Handler::spend("QarsiDeceiver.QarsiDeceiverManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    let o = game.obj(cx.on);
    if cx.spell() {
        return o.face_down;
    }
    matches!(cx.ability_def(game), Some(AbilityDef::TurnFaceUpAbility { .. }))
        && matches!(o.face_down_as, Some(FaceDown::Manifest) | Some(FaceDown::Morph))
}

//! Brisela, Voice of Nightmares: "Your opponents can't cast spells with mana value 3 or less." (the meld card of
//! Gisela, the Broken Blade and Bruna, the Fading Light)

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("BriselaVoiceOfNightmares.BriselaVoiceOfNightmaresCantCastEffect", Rule::CantCastOnStack(cant_cast))];

/// `CAST_SPELL_LATE`: an opponent's spell on the stack with a mana value below 4
fn cant_cast(_game: &Game, cx: &StaticCx, seat: Seat, _id: ObjId, face: &CardDef) -> bool {
    cx.opponent(seat) && face.mana_value() < 4
}

//! Akoum Firebird: "Flying, haste. Akoum Firebird attacks each combat if able. Landfall — Whenever a land you control
//! enters, you may pay {4}{R}{R}. If you do, return Akoum Firebird from your graveyard to the battlefield."

use super::*;

/// `checkTrigger`: a land entering under your control, from its owner's graveyard: the landfall trigger itself
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AkoumFirebird.AkoumFirebirdLandfallAbility",
    "new LandfallAbility(Zone.GRAVEYARD, $0, $1)",
    any_batch,
)];

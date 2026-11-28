//! Felhide Spiritbinder: "Inspired — Whenever Felhide Spiritbinder becomes untapped, you may pay {1}{R}. If you do,
//! create a token that's a copy of another target creature, except it's an enchantment in addition to its other
//! types. It gains haste. Exile it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FelhideSpiritbinder.FelhideSpiritbinderEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, ENCHANTMENT, true)` of the target (or as it last was), each token exiled at the
/// beginning of the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::cogwork_assembler::copy_with_haste_exiled(game, d, cx, CardType::Enchantment)
}

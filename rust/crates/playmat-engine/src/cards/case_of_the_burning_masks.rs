//! Case of the Burning Masks: "When this Case enters, it deals 3 damage to target creature an opponent controls. To
//! solve — Three or more sources you controlled dealt damage this turn. (If unsolved, solve at the beginning of your
//! end step.) Solved — Sacrifice this Case: Exile the top three cards of your library. Choose one of them. You may
//! play that card this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("CaseOfTheBurningMasks.CaseOfTheBurningMasksCondition", three_sources)];

/// `CaseOfTheBurningMasksWatcher`: the different sources (as the objects they were) you controlled that dealt damage
fn three_sources(game: &Game, you: Seat, _: ObjId) -> bool {
    let mut seen: Vec<(ObjId, u32)> = Vec::new();
    for d in game.watch.damage.iter().filter(|d| d.source_controller == you) {
        if !seen.contains(&(d.source, d.source_zcc)) {
            seen.push((d.source, d.source_zcc));
        }
    }
    seen.len() >= 3
}

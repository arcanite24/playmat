//! Sorin, Vengeful Bloodlord: "-X: Return target creature card with mana value X from your graveyard to the
//! battlefield. That creature is a Vampire in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SorinVengefulBloodlord.SorinVengefulBloodlordEffect", apply)];

/// The returned creature is a Vampire for as long as it stays (`Duration.Custom`, layer 4)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    if game.obj(c).zone == Zone::Battlefield {
        let zcc = game.obj(c).zcc;
        game.subtypes_while_there.push((c, zcc, "Vampire".to_string()));
    }
    Some(())
}

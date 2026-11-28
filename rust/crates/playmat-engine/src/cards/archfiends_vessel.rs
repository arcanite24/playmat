//! Archfiend's Vessel: "Lifelink. When Archfiend's Vessel enters, if it entered from your graveyard or you cast it
//! from your graveyard, exile it. If you do, create a 5/5 black Demon creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ArchfiendsVessel.ArchfiendsVesselAbility",
        "new EntersBattlefieldTriggeredAbility(new ArchfiendsVesselEffect())",
        from_graveyard,
    ),
    Handler::effect("ArchfiendsVessel.ArchfiendsVesselEffect", exile_for_demon).tokens(&["DemonToken"]),
];

/// `checkTrigger` only (it has no intervening "if"): it entered from a graveyard, or as a spell cast from one
/// (`SpellsCastWatcher.getSpellsCastFromGraveyardThisTurn`)
fn from_graveyard(game: &Game, t: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability) {
        return true;
    }
    let o = game.obj(t.card);
    o.entered_from == Some(Zone::Graveyard) || (o.cast && o.cast_from == Some(Zone::Graveyard))
}

/// The source, if it's still the permanent that triggered, is exiled; if it went, a 5/5 Demon
fn exile_for_demon(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let id = cx.source_id();
    game.move_to(id, Zone::Exile);
    if game.obj(id).zone == Zone::Exile {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}

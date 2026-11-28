//! Orcrist, Goblin-cleaver: "Whenever equipped creature deals combat damage to a player, choose a creature type.
//! Create a Treasure token for each creature you control of that type."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OrcristGoblinCleaver.OrcristGoblinCleaverEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken())"])];

/// `ChoiceCreatureType`, asked of you; then one Treasure (`CreateTokenEffect(TreasureToken, count)`) per creature you
/// control of that type
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let count = game
        .battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == you && game.has_type(**o, CardType::Creature) && game.has_subtype(**o, &chosen)
        })
        .count() as i32;
    if count > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(count);
        }
        cx.run_on(game, d, &[e], Vec::new());
    }
    Some(())
}

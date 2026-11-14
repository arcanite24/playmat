//! Glissa, Herald of Predation: "At the beginning of combat on your turn, choose one — • Incubate 2 twice.
//! • Transform all Incubator tokens you control. • Phyrexians you control gain first strike and deathtouch until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GlissaHeraldOfPredation.GlissaHeraldOfPredationIncubateEffect", incubate)
        .tokens(&["IncubatorToken"]),
    Handler::effect("GlissaHeraldOfPredation.GlissaHeraldOfPredationTransformEffect", transform_all),
];

/// `IncubateEffect.doIncubate(2)` twice
fn incubate(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for _ in 0..2 {
        let e = EffectDef::IncubateEffect { amount: 2, token: cx.tokens[0].clone() };
        cx.run(game, d, &[e]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

/// each Incubator token you control, in battlefield order (`getActivePermanents`), transforms
fn transform_all(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let incubators: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.obj(*o).token && game.has_subtype(*o, "Incubator"))
        .collect();
    for o in incubators {
        if game.obj(o).zone == Zone::Battlefield {
            transform(game, o);
        }
    }
    Some(())
}

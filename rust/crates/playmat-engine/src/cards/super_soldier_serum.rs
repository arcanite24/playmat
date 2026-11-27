//! Super-Soldier Serum: "Enchanted creature gets +2/+2, has first strike and vigilance, and is a legendary Soldier in
//! addition to its other types. Whenever enchanted creature attacks or blocks, attach any number of target Equipment
//! you control to it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::continuous("SuperSoldierSerum.SuperSoldierSerumTypeEffect", enchanted, Layer::Type(soldier))
        .and_layers(&[Layer::SuperType(legendary)]),
    Handler::effect("SuperSoldierSerum.SuperSoldierSerumAttachEffect", attach),
];

fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

fn soldier(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    vec!["Soldier".into()]
}

fn legendary(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    vec!["LEGENDARY".into()]
}

fn attach(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let Some(creature) = game.attached(cx.source_id()) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let czcc = game.obj(creature).zcc;
    for eq in cx.objs(0) {
        if game.obj(eq).zone == Zone::Battlefield && eq != creature {
            game.attach_to(eq, creature, czcc);
        }
    }
    Some(())
}

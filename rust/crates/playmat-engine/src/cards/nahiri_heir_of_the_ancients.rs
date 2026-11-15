//! Nahiri, Heir of the Ancients: "+1: Create a 1/1 white Kor Warrior creature token. You may attach an Equipment
//! you control to it. −2: Look at the top six cards of your library. You may reveal a Warrior or Equipment card from
//! among them and put it into your hand. Put the rest on the bottom of your library in a random order. −3: Nahiri
//! deals damage to target creature or planeswalker equal to twice the number of Equipment you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NahiriHeirOfTheAncients.NahiriHeirOfTheAncientsEffect", apply)
    .effects(&["new CreateTokenEffect(new KorWarriorToken())"])];

/// The Kor Warrior; with an Equipment you control, "Attach an equipment to the token?", the token chosen when
/// there are more, then an Equipment you control (not targeted) attached to it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    let equipment = |game: &Game| -> Vec<ObjId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|e| game.obj(*e).controller == you && game.has_subtype(*e, "Equipment"))
            .collect()
    };
    if made.is_empty() || equipment(game).is_empty() || !choose_use(game, d, you, "Attach an equipment to the token?")? {
        return Some(());
    }
    let mut token = made[0];
    if made.len() > 1 {
        let Some(&t) = choose_cards(game, d, you, &made, &made, 1, 1, "token")?.first() else { return Some(()) };
        token = t;
    }
    let possible = equipment(game);
    let Some(&e) = choose_cards(game, d, you, &possible, &possible, 1, 1, "Equipment you control")?.first() else {
        return Some(());
    };
    if game.obj(token).zone == Zone::Battlefield && game.attached(e) != Some(token) {
        let zcc = game.obj(token).zcc;
        game.attach_to(e, token, zcc);
    }
    Some(())
}

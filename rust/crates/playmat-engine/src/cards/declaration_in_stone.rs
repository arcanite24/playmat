//! Declaration in Stone: "Exile target creature and all other creatures its controller controls with the same name
//! as that creature. That player investigates for each nontoken creature exiled this way."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DeclarationInStone.DeclarationInStoneEffect", apply).tokens(&["ClueArtifactToken"])];

/// The target, and unless it's face down (no name) every other creature its controller controls with its name
/// (`CardUtil.haveSameNames`), exiled together; the count is Java's: one for the target if it isn't a token, and one
/// for each other creature if the target isn't a token. Then its controller investigates that many times.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(target).controller;
    let target_token = game.obj(target).token;
    let name = game.obj(target).name().to_string();
    let mut exiled = vec![target];
    let mut non_token = usize::from(!target_token);
    if !game.obj(target).face_down && !name.is_empty() {
        let others: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| {
                *o != target
                    && game.obj(*o).controller == p
                    && game.has_type(*o, CardType::Creature)
                    && !game.obj(*o).face_down
                    && game.obj(*o).name() == name
            })
            .collect();
        for o in others {
            exiled.push(o);
            if !target_token {
                non_token += 1;
            }
        }
    }
    for o in &exiled {
        game.move_to(*o, Zone::Exile);
    }
    if non_token > 0 {
        let token = cx.tokens.first()?.clone();
        let e = EffectDef::CreateTokenEffect {
            token,
            amount: crate::card::Amount::Fixed(non_token as i32),
            tapped_and_attacking: false,
            tapped: false,
            attacking: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            haste_until_end_of_turn: false,
            also: None,
            attach_source: None,
        };
        cx.run_as(game, d, &[e], p);
    }
    Some(())
}

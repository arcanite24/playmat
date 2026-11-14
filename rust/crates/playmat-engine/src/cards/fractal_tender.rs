//! Fractal Tender: "Ward {2}. Increment. At the beginning of each end step, if you put a counter on this creature
//! this turn, create a 0/0 green and blue Fractal creature token and put three +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("FractalTender.FractalTenderCondition", you_put_a_counter)];

/// `FractalTenderWatcher.check`: you put a counter on the source this turn, as the object it is now
fn you_put_a_counter(game: &Game, you: Seat, source: ObjId) -> bool {
    let zcc = game.obj(source).zcc;
    game.watch.counters_put.iter().any(|(by, o, z, _, n, _)| *by == Some(you) && *o == source && *z == zcc && *n > 0)
}

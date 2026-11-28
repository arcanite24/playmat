package mage.server.career;

import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;

import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

/** A game the Rust host played is tallied by the host and paid as one Java played (RustHost.summarize) */
class CareerTallyHostTest {

    @Test
    void aHostsTallyEndsAsAWatchersWould() {
        UUID game = UUID.randomUUID();
        UUID me = UUID.randomUUID();
        String json = "{\"counters\": {\"spells\": 6, \"my_turns\": 4, \"damage_to_opponents\": 22, \"creature_spells\": 0, \"damage_taken\": 16},"
                + " \"moments\": [{\"id\": \"big_hit\", \"value\": 12, \"turn\": 7}, {\"id\": \"storm\", \"value\": 2, \"turn\": 3}],"
                + " \"lowestLife\": 4, \"life\": 9, \"startingLife\": 20,"
                + " \"opponents\": [{\"life\": -11, \"poison\": 0, \"library\": 30}]}";
        CareerTally tally = CareerTally.fromHost(game, me, JsonParser.parseString(json).getAsJsonObject());
        assertSame(tally, CareerTally.of(game, me), "kept for the game's end");
        tally.finish(null, true, 8);
        assertEquals(6, tally.get("spells"));
        assertEquals(1, tally.get(CareerTally.WINS));
        assertEquals(9, tally.get("life_at_end"));
        assertEquals(1, tally.get("won_from_five_life"), "the lowest life the host saw");
        assertEquals(1, tally.get("won_by_turn_four"));
        assertEquals(1, tally.get("won_without_creatures"));
        assertEquals(1, tally.get("won_overkill"), "the opponent's life at the end");
        assertEquals(0, tally.get("won_flawless"));
        // the host's big hit, then the end's comeback and fast win; a storm of two isn't a feat
        assertEquals(java.util.Arrays.asList("big_hit", "comeback", "fast_win"),
                tally.moments().stream().map(m -> m.id).collect(java.util.stream.Collectors.toList()));
        assertEquals(tally, CareerTally.takeAll(game).get(me));
        assertNull(CareerTally.of(game, me));
    }
}

package mage.server.career;

import mage.server.ladder.BotRoster;
import mage.server.ladder.LadderStore;
import mage.server.ladder.RankedService;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** A Career sealed or draft run moves the Limited rank when it ends, not before (#435). */
public class CareerLimitedRankTest {

    private static final long NOW = 1_800_000_000_000L;

    private Path careerFile;
    private CareerStore career;
    private LadderStore ladder;
    private RankedService ranked;

    @BeforeEach
    void open() throws Exception {
        careerFile = Files.createTempFile("limited-rank", ".db");
        career = new CareerStore("jdbc:sqlite:" + careerFile);
        CareerStore.use(career);
        ladder = new LadderStore("jdbc:sqlite::memory:");
        ranked = RankedService.forTests(ladder, BotRoster.get());
        RankedService.use(ranked);
    }

    @AfterEach
    void close() throws Exception {
        RankedService.use(null);
        CareerStore.use(null);
        career.close();
        ladder.close();
        Files.deleteIfExists(careerFile);
    }

    @Test
    void theRankMovesOnceTheRunIsOver() throws Exception {
        CareerService.ensureProfile(career, "neri", NOW);
        CareerStore.RunRow row = new CareerStore.RunRow();
        row.id = "run-1";
        row.kind = "limited";
        row.started = NOW;
        row.state = "active";
        row.data = "{\"kind\":\"sealed\",\"shape\":\"classic\",\"setCode\":\"FDN\",\"setName\":\"Foundations\",\"phase\":\"play\"}";
        career.putRun("neri", row);
        int season = ranked.season(NOW);

        CareerProgress.CareerGameResult first = new CareerProgress.CareerGameResult();
        CareerLimited.get().finished(career, "neri", "run-1", true, first, NOW);
        CareerLimited.get().finished(career, "neri", "run-1", true, new CareerProgress.CareerGameResult(), NOW);
        assertNull(ladder.standing("neri", RankedService.LIMITED, RankedService.OPEN, season), "the run isn't over at 2-0");
        assertTrue(first.mode.lines.stream().noneMatch(line -> line.startsWith("Limited rank")));

        CareerProgress.CareerGameResult last = new CareerProgress.CareerGameResult();
        CareerLimited.get().finished(career, "neri", "run-1", true, last, NOW);
        LadderStore.Standing standing = ladder.standing("neri", RankedService.LIMITED, RankedService.OPEN, season);
        assertEquals(3, standing.step, "3-0 is three pips");
        assertTrue(last.mode.lines.contains("Limited rank: +3 pips, Bronze 4 (3/4 pips)."), String.valueOf(last.mode.lines));
        assertEquals("won", ladder.game("limited-run-1").status);
    }
}

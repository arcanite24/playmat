package mage.server.solo;

import java.util.ArrayList;
import java.util.List;

/** What the web client sees of solo saves and batch jobs. */
public final class SoloViews {

    private SoloViews() {
    }

    /** A saved solo game. */
    public static class SoloSave {
        public String id;
        public long created;
        public long updated;
        /** "auto" (saved as it went) or "kept" (saved and left on purpose) */
        public String kind;
        public boolean career;
        public int turn;
        public String title;
        public String opponents;
        public String gameType;
        /** the game is still going: rejoin it instead */
        public boolean running;

        public static SoloSave ofRow(SoloStore.SavedRow row, boolean running) {
            SoloSave save = new SoloSave();
            save.id = row.id;
            save.created = row.created;
            save.updated = row.updated;
            save.kind = row.kind;
            save.career = row.career;
            save.turn = row.turn;
            save.title = row.title;
            save.opponents = row.opponents;
            save.gameType = row.gameType;
            save.running = running;
            return save;
        }
    }

    /** Where a resumed or forked game is. */
    public static class SoloTable {
        public String tableId;
    }

    /** An AI-against-AI batch. */
    public static class BatchJob {
        public String id;
        public String user;
        public String kind;
        /** queued, running, done, cancelled or failed */
        public String status;
        public long created;
        public Long started;
        public Long finished;
        public int games;
        public int done;
        public String nameA;
        public String nameB;
        public int skillA;
        public int skillB;
        public long seed;
        public int maxTurns;
        public BatchRunner.JobResult result;
        public String error;

        /** {@code withGames}: every game's line, not only the score */
        public static BatchJob of(SoloStore.JobRow row, boolean withGames) {
            BatchJob job = new BatchJob();
            job.id = row.id;
            job.user = row.user;
            job.kind = row.kind;
            job.status = row.status;
            job.created = row.created;
            job.started = row.started;
            job.finished = row.finished;
            job.games = row.games;
            job.done = row.done;
            job.error = row.error;
            try {
                BatchRunner.JobSpec spec = BatchRunner.spec(row);
                job.nameA = spec.nameA;
                job.nameB = spec.nameB;
                job.skillA = spec.skillA;
                job.skillB = spec.skillB;
                job.seed = spec.seed == null ? 0 : spec.seed;
                job.maxTurns = spec.maxTurns;
            } catch (RuntimeException ignored) {
                // an unreadable spec still lists
            }
            job.result = BatchRunner.result(row);
            if (!withGames) {
                job.result.games.clear();
            }
            return job;
        }
    }

    /** The batch runner as staff see it. */
    public static class BatchStatus {
        public int concurrency;
        public int gamesPerDay;
        public int gamesInFlight;
        public List<BatchJob> jobs = new ArrayList<>();
    }

    /** What the player of a solo game may still ask for (S010, S011). */
    public static class Assists {
        /** whether a hint (the AI's advice) is on offer now */
        public boolean hints;
        /** hints left in the game; null where they aren't counted (free play) */
        public Integer hintsLeft;
        /** whether a turn can be taken back now */
        public boolean takeBack;
        /** take-backs left; null where they aren't counted (free play) */
        public Integer takeBacksLeft;
        /** a Career game: help is counted and halves the pay */
        public boolean career;
        /** what has helped already in this Career game, or null */
        public String assisted;
        /** why nothing is on offer, when nothing is */
        public String why;
    }

    /** A turn of a solo game that can be played again (S013, S014). */
    public static class SoloTurn {
        public int turn;
        /** whose turn it is */
        public String active;
        public long at;

        public static SoloTurn of(SoloStore.TurnRow row) {
            SoloTurn turn = new SoloTurn();
            turn.turn = row.turn;
            turn.active = row.active;
            turn.at = row.at;
            return turn;
        }
    }
}

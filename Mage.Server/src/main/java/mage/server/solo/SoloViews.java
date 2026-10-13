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
}

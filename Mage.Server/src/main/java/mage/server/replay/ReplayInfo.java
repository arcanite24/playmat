package mage.server.replay;

import java.util.ArrayList;
import java.util.List;

/**
 * What a replay list shows about one recorded game; also the first line of the replay file.
 */
public class ReplayInfo {

    public static class ReplayPlayer {
        public String name;
        public boolean human;

        public ReplayPlayer() {
        }

        public ReplayPlayer(String name, boolean human) {
            this.name = name;
            this.human = human;
        }
    }

    public int version = 1;
    public String gameId;
    public String tableId;
    public String tableName;
    public String gameType;
    public String deckType;
    public List<ReplayPlayer> players = new ArrayList<>();
    public long startedAt;
    /** 0 while the game is being recorded */
    public long endedAt;
    /** the server's end message, such as "Player Ana is the winner" */
    public String result;
    public int turns;
    /** recorded steps: board snapshots and log lines */
    public int events;
    /** the recording stopped early (the game outgrew the limit, or the server stopped) */
    public boolean truncated;
    /** an AI-against-AI game kept from a player's batch (matchup lab): not in the shared list, only linked from it */
    public String batchOwner;
}

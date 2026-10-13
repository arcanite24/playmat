package mage.server.solo;

import com.google.gson.Gson;
import mage.cards.Card;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.PhaseStep;
import mage.game.Game;
import mage.game.events.Listener;
import mage.game.events.TableEvent;
import mage.players.Player;
import mage.server.career.CareerSetup;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * The puzzle miner (S031): watches an AI-against-AI game, writes down each turn's position at the start of its first
 * main phase, and when a player wins on their own turn, that turn's position becomes a puzzle: the AI found lethal
 * there, so it can be solved. Each is rated by depth, the spells from hand it took plus one; a position won by
 * attacking with what was already on board (depth 1) is too easy to keep.
 */
public final class PuzzleMiner {

    private static final Logger logger = Logger.getLogger(PuzzleMiner.class);
    public static final int MIN_DEPTH = 2;

    private static final class Position {
        UUID active;
        PuzzleKit.PuzzleExport export;
    }

    private final Game game;
    private final Map<Integer, Position> turns = new HashMap<>();

    private PuzzleMiner(Game game) {
        this.game = game;
    }

    /** starts watching a game that hasn't started */
    public static PuzzleMiner attach(Game game) {
        PuzzleMiner miner = new PuzzleMiner(game);
        game.addTableEventListener((Listener<TableEvent>) event -> {
            if (event.getEventType() == TableEvent.EventType.UPDATE) {
                miner.look();
            }
        });
        return miner;
    }

    private void look() {
        try {
            if (game.getStep() == null || game.getStep().getType() != PhaseStep.PRECOMBAT_MAIN) {
                return;
            }
            int turn = game.getTurnNum();
            if (turns.containsKey(turn)) {
                return;
            }
            Position position = new Position();
            position.active = game.getActivePlayerId();
            position.export = PuzzleKit.capture(game, position.active, null, null, "win");
            turns.put(turn, position);
        } catch (RuntimeException e) {
            logger.debug("Miner: a position couldn't be read", e);
        }
    }

    /**
     * The puzzle the finished game left, if any, kept in the store.
     *
     * @return how many were kept (0 or 1)
     */
    public int keep(String user, String source) {
        CareerSetup.Puzzle puzzle = found();
        if (puzzle == null) {
            return 0;
        }
        SoloStore.MinedRow row = new SoloStore.MinedRow();
        row.id = puzzle.id;
        row.created = System.currentTimeMillis();
        row.user = user;
        row.depth = depth;
        row.source = source;
        row.puzzle = new Gson().toJson(puzzle);
        try {
            return SoloStore.get().putMined(row) ? 1 : 0;
        } catch (Exception e) {
            logger.warn("Miner: a puzzle couldn't be kept", e);
            return 0;
        }
    }

    private int depth;

    /** the winning turn's position as a puzzle, or null when the game didn't end in lethal worth keeping */
    CareerSetup.Puzzle found() {
        Position position = turns.get(game.getTurnNum());
        if (position == null || !position.export.left.isEmpty()) {
            return null;
        }
        Player winner = game.getPlayer(position.active);
        if (winner == null || !winner.hasWon()) {
            return null;
        }
        Player loser = null;
        for (Player player : game.getPlayers().values()) {
            if (!player.getId().equals(winner.getId())) {
                loser = player;
            }
        }
        if (loser == null || loser.getLife() > 0) {
            // a concession or an empty library isn't lethal
            return null;
        }
        CareerSetup.Puzzle puzzle = position.export.puzzle;
        depth = 1 + spellsUsed(puzzle.you.hand, winner);
        if (depth < MIN_DEPTH) {
            return null;
        }
        puzzle.name = "Lethal on turn " + game.getTurnNum();
        puzzle.hint = depth >= 4 ? "It takes " + (depth - 1) + " spells from your hand." : null;
        puzzle.series = "Mined";
        return puzzle;
    }

    public int depth() {
        return depth;
    }

    /** nonland cards that were in hand at the start of the turn and aren't now */
    private int spellsUsed(List<String> handBefore, Player winner) {
        List<String> left = new ArrayList<>();
        for (Card card : winner.getHand().getCards(game)) {
            left.add(card.getName());
        }
        int used = 0;
        for (String name : handBefore) {
            if (!left.remove(name)) {
                CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
                if (info == null || !info.getTypes().contains(mage.constants.CardType.LAND)) {
                    used++;
                }
            }
        }
        return used;
    }
}

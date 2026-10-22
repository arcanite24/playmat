package mage.server.solo;

import com.google.gson.Gson;
import mage.cards.Card;
import mage.cards.repository.CardRepository;
import mage.constants.SubType;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.game.permanent.PermanentToken;
import mage.players.Player;
import mage.server.career.CareerSetup;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

/**
 * Puzzles from boards (S030, S031): a game's position written as a puzzle (names, life, tapped permanents), the
 * checks a puzzle from a file must pass before it's played, and a stable id for a position.
 * <p>
 * A puzzle holds card names only, so what a name can't bring back is left out and listed: tokens, counters, auras
 * (which would come back unattached) and transformed cards.
 */
public final class PuzzleKit {

    /** the most cards a puzzle zone may hold */
    public static final int MAX_ZONE = 60;
    /** library cards kept from the top, so a draw in the puzzle draws what the game would have */
    public static final int LIBRARY_TOP = 3;

    /** A position, and what couldn't be kept of it. */
    public static class PuzzleExport {
        public CareerSetup.Puzzle puzzle;
        /** "a Soldier token", "counters on Hangarback Walker" */
        public List<String> left = new ArrayList<>();
    }

    private PuzzleKit() {
    }

    /**
     * The board as a puzzle for {@code youId} against the other player.
     *
     * @param objective "win" or "survive"; win by default
     */
    public static PuzzleExport capture(Game game, UUID youId, String name, String text, String objective) {
        PuzzleExport export = new PuzzleExport();
        CareerSetup.Puzzle puzzle = new CareerSetup.Puzzle();
        Player you = game.getPlayer(youId);
        Player opponent = null;
        for (UUID id : game.getState().getPlayersInRange(youId, game)) {
            if (!id.equals(youId)) {
                opponent = game.getPlayer(id);
                break;
            }
        }
        puzzle.objective = "survive".equals(objective) ? "survive" : "win";
        puzzle.name = name == null || name.trim().isEmpty() ? "Puzzle" : name.trim();
        puzzle.you = side(game, you, export.left, "your");
        puzzle.opponent = side(game, opponent, export.left, "the opponent's");
        puzzle.text = text == null || text.trim().isEmpty()
                ? ("survive".equals(puzzle.objective) ? "Survive their turn." : "Your opponent is at "
                + (puzzle.opponent.life == null ? 20 : puzzle.opponent.life) + " life. Win this turn.")
                : text.trim();
        if ("survive".equals(puzzle.objective)) {
            puzzle.firstTurn = "opponent";
        }
        puzzle.id = positionId(puzzle);
        export.puzzle = puzzle;
        return export;
    }

    private static CareerSetup.CareerPuzzleSide side(Game game, Player player, List<String> left, String whose) {
        CareerSetup.CareerPuzzleSide side = new CareerSetup.CareerPuzzleSide();
        side.hand = new ArrayList<>();
        side.battlefield = new ArrayList<>();
        side.graveyard = new ArrayList<>();
        side.library = new ArrayList<>();
        if (player == null) {
            return side;
        }
        side.life = player.getLife();
        for (Card card : player.getHand().getCards(game)) {
            side.hand.add(card.getName());
        }
        for (Card card : player.getGraveyard().getCards(game)) {
            side.graveyard.add(card.getName());
        }
        for (Card card : player.getLibrary().getTopCards(game, LIBRARY_TOP)) {
            side.library.add(card.getName());
        }
        for (Permanent permanent : game.getBattlefield().getAllActivePermanents(player.getId())) {
            if (permanent instanceof PermanentToken) {
                left.add("a " + permanent.getName() + " token (" + whose + ")");
                continue;
            }
            if (permanent.isTransformed() || permanent.isFaceDown(game)) {
                left.add(permanent.getName() + ", transformed or face down (" + whose + ")");
                continue;
            }
            if (permanent.hasSubtype(SubType.AURA, game)) {
                left.add(permanent.getName() + ", an aura (" + whose + ")");
                continue;
            }
            if (!permanent.getCounters(game).isEmpty()) {
                left.add("counters on " + permanent.getName() + " (" + whose + ")");
            }
            side.battlefield.add(permanent.getName() + (permanent.isTapped() ? "|tapped" : ""));
        }
        return side;
    }

    /** the same position always gets the same id */
    public static String positionId(CareerSetup.Puzzle puzzle) {
        String key = new Gson().toJson(new Object[]{puzzle.objective, puzzle.you, puzzle.opponent});
        try {
            byte[] hash = MessageDigest.getInstance("SHA-1").digest(key.getBytes(StandardCharsets.UTF_8));
            StringBuilder id = new StringBuilder("p-");
            for (int i = 0; i < 6; i++) {
                id.append(String.format("%02x", hash[i]));
            }
            return id.toString();
        } catch (NoSuchAlgorithmException e) {
            return "p-" + Integer.toHexString(key.hashCode());
        }
    }

    /** what's wrong with a puzzle from a file, or null when it can be played */
    public static String problem(CareerSetup.Puzzle puzzle) {
        if (puzzle == null || puzzle.you == null || puzzle.opponent == null) {
            return "A puzzle needs both sides.";
        }
        String objective = puzzle.problem();
        if (objective != null) {
            return "The puzzle is wrong: " + objective + ".";
        }
        if (puzzle.name != null && puzzle.name.length() > 80 || puzzle.text != null && puzzle.text.length() > 400
                || puzzle.hint != null && puzzle.hint.length() > 400) {
            return "The puzzle's name or text is too long.";
        }
        for (CareerSetup.CareerPuzzleSide side : new CareerSetup.CareerPuzzleSide[]{puzzle.you, puzzle.opponent}) {
            if (side.life != null && (side.life < 1 || side.life > 999)) {
                return "Life is 1 to 999.";
            }
            for (List<String> zone : zones(side)) {
                if (zone == null) {
                    continue;
                }
                if (zone.size() > MAX_ZONE) {
                    return "A zone holds at most " + MAX_ZONE + " cards.";
                }
                for (String entry : zone) {
                    String name = entry == null ? "" : entry.split("\\|")[0].trim();
                    if (name.isEmpty() || CardRepository.instance.findPreferredCoreExpansionCard(name) == null) {
                        return "There's no card named " + name + ".";
                    }
                }
            }
        }
        if (puzzle.you.hand == null && puzzle.you.battlefield == null) {
            return "Your side of the puzzle is empty.";
        }
        return null;
    }

    private static List<List<String>> zones(CareerSetup.CareerPuzzleSide side) {
        List<List<String>> zones = new ArrayList<>();
        zones.add(side.hand);
        zones.add(side.battlefield);
        zones.add(side.graveyard);
        zones.add(side.library);
        return zones;
    }
}

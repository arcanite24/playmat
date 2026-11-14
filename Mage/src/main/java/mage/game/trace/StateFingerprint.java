package mage.game.trace;

import mage.cards.Card;
import mage.counters.Counter;
import mage.counters.Counters;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.game.stack.StackObject;
import mage.players.Player;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.UUID;
import java.util.stream.Collectors;

/**
 * The game as plain text that two engines can agree on, and its hash. Players are named by seat (their place in the
 * turn order, 0 first) and objects by card name, so two runs of the same seeded game match even though object ids are
 * random. The format is specified in docs/rust/StateFingerprint.md; change both together and bump {@link #VERSION}.
 */
public final class StateFingerprint {

    public static final int VERSION = 2;

    private StateFingerprint() {
    }

    /**
     * The players in seating order, seat 0 first. Not {@code getPlayerList()}: that list is circular and starts its
     * iteration at the current player, so its order turns with the game.
     */
    public static List<UUID> seats(Game game) {
        return new ArrayList<>(game.getState().getPlayers().keySet());
    }

    public static String text(Game game) {
        List<UUID> seats = seats(game);
        StringBuilder out = new StringBuilder();
        out.append("v").append(VERSION).append('\n');
        out.append("turn ").append(game.getTurnNum())
                .append(" step ").append(game.getTurnStepType() == null ? "-" : game.getTurnStepType().name())
                .append(" active ").append(seat(seats, game.getActivePlayerId()))
                .append(" priority ").append(seat(seats, game.getPriorityPlayerId()))
                .append('\n');
        for (int i = 0; i < seats.size(); i++) {
            Player player = game.getPlayer(seats.get(i));
            if (player == null) {
                continue;
            }
            out.append("player ").append(i)
                    .append(" life ").append(player.getLife())
                    .append(" counters ").append(counters(player.getCountersAsCopy()))
                    .append(player.hasLost() ? " lost" : "")
                    .append(player.hasWon() ? " won" : "")
                    .append('\n');
            out.append(" hand ").append(sortedNames(game, player.getHand())).append('\n');
            out.append(" library ").append(names(game, player.getLibrary().getCardList())).append('\n');
            out.append(" graveyard ").append(names(game, player.getGraveyard())).append('\n');
        }
        List<String> exile = game.getState().getExile().getAllCards(game).stream()
                .map(Card::getName).sorted().collect(Collectors.toList());
        out.append("exile ").append(String.join("|", exile)).append('\n');
        List<String> battlefield = new ArrayList<>();
        for (Permanent permanent : game.getState().getBattlefield().getAllPermanents()) {
            battlefield.add(seat(seats, permanent.getControllerId())
                    + " " + permanent.getName()
                    + (permanent.isTapped() ? " T" : " U")
                    + (permanent.isFaceDown(game) ? " facedown" : "")
                    + " " + permanent.getPower().getValue() + "/" + permanent.getToughness().getValue()
                    + " dmg " + permanent.getDamage()
                    + (permanent.isAttacking() ? " attacking" : "")
                    + (permanent.getBlocking() > 0 ? " blocking" : "")
                    + " counters " + counters(permanent.getCounters(game)));
        }
        battlefield.sort(null);
        for (String line : battlefield) {
            out.append("bf ").append(line).append('\n');
        }
        for (StackObject object : game.getState().getStack()) { // top first
            out.append("stack ").append(seat(seats, object.getControllerId())).append(' ')
                    .append(withoutObjectIds(object.getName())).append('\n');
        }
        return out.toString();
    }

    public static String hash(Game game) {
        return hashText(text(game));
    }

    /** FNV-1a, 64 bits, over the UTF-8 bytes; 16 lowercase hex digits */
    public static String hashText(String text) {
        long hash = 0xcbf29ce484222325L;
        for (byte b : text.getBytes(StandardCharsets.UTF_8)) {
            hash ^= (b & 0xff);
            hash *= 0x100000001b3L;
        }
        return String.format("%016x", hash);
    }

    private static String seat(List<UUID> seats, UUID playerId) {
        int index = playerId == null ? -1 : seats.indexOf(playerId);
        return index < 0 ? "-" : Integer.toString(index);
    }

    private static String counters(Counters counters) {
        if (counters == null || counters.isEmpty()) {
            return "-";
        }
        Map<String, Integer> sorted = new TreeMap<>();
        for (Counter counter : counters.values()) {
            if (counter.getCount() != 0) {
                sorted.put(counter.getName(), counter.getCount());
            }
        }
        if (sorted.isEmpty()) {
            return "-";
        }
        return sorted.entrySet().stream().map(e -> e.getKey() + "=" + e.getValue()).collect(Collectors.joining(","));
    }

    private static String names(Game game, Iterable<UUID> ids) {
        List<String> names = new ArrayList<>();
        for (UUID id : ids) {
            names.add(name(game, id));
        }
        return String.join("|", names);
    }

    private static String sortedNames(Game game, Iterable<UUID> ids) {
        List<String> names = new ArrayList<>();
        for (UUID id : ids) {
            names.add(name(game, id));
        }
        names.sort(null);
        return String.join("|", names);
    }

    private static final java.util.regex.Pattern LOG_NAME =
            java.util.regex.Pattern.compile("<font color='[^']*' object_id='[^']*'>([^<]*)</font> \\[[^\\]]*\\]");

    /**
     * A stack object's name with each object's log name ({@code GameLog.getColoredObjectIdName}) as just the name: a
     * stack ability's rule can carry one (cipher's "you may cast a copy of ..."), and its object id is random.
     */
    static String withoutObjectIds(String name) {
        return name.indexOf("object_id='") < 0 ? name : LOG_NAME.matcher(name).replaceAll("$1");
    }

    private static String name(Game game, UUID id) {
        Card card = game.getCard(id);
        return card == null ? "?" : card.getName();
    }
}

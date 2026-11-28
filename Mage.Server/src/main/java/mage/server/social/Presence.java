package mage.server.social;

import mage.view.RoomUsersView;
import mage.view.UsersView;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

/**
 * Who shows as online (#229). A player may appear offline ("hidden") or say they are looking to play; the choice is
 * kept per account ({@link PresenceStore}) until they change it.
 * <p>
 * A hidden player is left out of the online list for everyone but themselves and staff, so their friends see them
 * offline. Their joining and leaving the lobby chat isn't announced, and whispers to them are refused like whispers to
 * someone who isn't here, except from players they whispered themselves this session (so a conversation they start
 * still works). What they do in public (chat in the lobby, a seat at a table) still shows: hidden is about presence,
 * not about being invisible while playing.
 */
public final class Presence {

    public static final String ONLINE = "online";
    public static final String HIDDEN = "hidden";
    /** "Looking to play": shown beside the name in the online list */
    public static final String LOOKING = "looking";

    /** per hidden user: the names they whispered this session (lower case), who may answer them */
    private static final ConcurrentMap<UUID, Set<String>> WHISPERED = new ConcurrentHashMap<>();
    static final int MAX_WHISPERED = 200;

    private Presence() {
    }

    /** A status as stored: one of the three, anything else counts as online. */
    public static String normalize(String status) {
        String text = status == null ? "" : status.trim().toLowerCase(Locale.ROOT);
        return HIDDEN.equals(text) || LOOKING.equals(text) ? text : ONLINE;
    }

    public static boolean known(String status) {
        String text = status == null ? "" : status.trim().toLowerCase(Locale.ROOT);
        return ONLINE.equals(text) || HIDDEN.equals(text) || LOOKING.equals(text);
    }

    /** The player's chosen status; online when nothing is stored (or the store is down). */
    public static String statusOf(String name) {
        PresenceStore store = PresenceStore.get();
        return store == null ? ONLINE : store.status(name);
    }

    public static boolean isHidden(String name) {
        return HIDDEN.equals(statusOf(name));
    }

    /**
     * The online list as {@code viewer} may see it: hidden players are left out, except the viewer themselves and
     * for staff (who see everyone, the hidden marked as such).
     */
    public static List<UsersView> visible(List<UsersView> users, String viewer, boolean staff) {
        if (staff) {
            return users;
        }
        List<UsersView> shown = new ArrayList<>(users.size());
        for (UsersView user : users) {
            if (!HIDDEN.equals(user.getStatus()) || (viewer != null && viewer.equalsIgnoreCase(user.getUserName()))) {
                shown.add(user);
            }
        }
        return shown;
    }

    /** {@link #visible(List, String, boolean)} for each room's list. */
    public static List<RoomUsersView> visibleRooms(List<RoomUsersView> rooms, String viewer, boolean staff) {
        if (staff || rooms == null) {
            return rooms;
        }
        List<RoomUsersView> shown = new ArrayList<>(rooms.size());
        for (RoomUsersView room : rooms) {
            shown.add(new RoomUsersView(visible(room.getUsersView(), viewer, false), room.getNumberActiveGames(),
                    room.getNumberGameThreads(), room.getNumberMaxGames()));
        }
        return shown;
    }

    /** A whisper from {@code fromName} may reach {@code toName} (whose session is {@code toUserId}). */
    public static boolean whisperReaches(String fromName, UUID toUserId, String toName) {
        if (!isHidden(toName)) {
            return true;
        }
        Set<String> answering = toUserId == null || fromName == null ? null : WHISPERED.get(toUserId);
        return answering != null && answering.contains(fromName.toLowerCase(Locale.ROOT));
    }

    /** Someone whispered {@code toName}: when they are hidden, that player may answer them. */
    public static void whispered(UUID fromUserId, String fromName, String toName) {
        if (fromUserId == null || toName == null || !isHidden(fromName)) {
            return;
        }
        Set<String> names = WHISPERED.computeIfAbsent(fromUserId, id -> Collections.newSetFromMap(new ConcurrentHashMap<>()));
        if (names.size() < MAX_WHISPERED) {
            names.add(toName.toLowerCase(Locale.ROOT));
        }
    }

    /** The session ended. */
    public static void forget(UUID userId) {
        if (userId != null) {
            WHISPERED.remove(userId);
        }
    }
}

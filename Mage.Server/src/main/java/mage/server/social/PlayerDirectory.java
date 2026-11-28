package mage.server.social;

import mage.server.AuthorizedUserRepository;

import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.function.Function;
import java.util.function.Predicate;

/**
 * The server's registered players (#195), for finding someone who isn't online: names, Career levels when Career is
 * on, and whether each is online. Nothing else about an account is listed (no email, no last sign-in), the search
 * matches names only, and accounts that can't sign in are left out. A hidden player (#229) shows as offline, except
 * to themselves and to staff. Staff can turn the directory off for the server (RuntimeSettings.DIRECTORY).
 */
public final class PlayerDirectory {

    public static final int PAGE_SIZE = 50;
    public static final int MAX_QUERY = 40;

    /** One player in the directory. */
    public static class DirectoryPlayer {
        public String name;
        /** the progression level; null without Career or before the player's first counted game */
        public Integer level;
        public boolean online;
        /** while online: "online", "looking" (to play), or "hidden" (only staff and the player see that one) */
        public String status;
    }

    /** One page of the directory. */
    public static class DirectoryPage {
        /** false when the server doesn't list its players (turned off, or no accounts): the rest is empty */
        public boolean enabled;
        public List<DirectoryPlayer> players = new ArrayList<>();
        /** players matching the search, every page together */
        public long total;
        public int offset;
        public int pageSize = PAGE_SIZE;
    }

    private PlayerDirectory() {
    }

    public static DirectoryPage disabled() {
        return new DirectoryPage();
    }

    /**
     * @param online whether a player is connected right now (whatever their status)
     * @param levels the levels of these players, or null without Career
     */
    public static DirectoryPage page(AuthorizedUserRepository accounts, String query, int offset, String viewer, boolean staff,
                                     Predicate<String> online, Function<Collection<String>, Map<String, Integer>> levels) {
        String search = query == null ? "" : query.trim();
        if (search.length() > MAX_QUERY) {
            search = search.substring(0, MAX_QUERY);
        }
        int start = Math.max(0, offset);
        DirectoryPage page = new DirectoryPage();
        page.enabled = true;
        page.offset = start;
        page.total = accounts.countActiveNames(search);
        List<String> names = accounts.listActiveNames(search, start, PAGE_SIZE);
        Map<String, Integer> found = levels == null || names.isEmpty() ? Collections.emptyMap() : levels.apply(names);
        for (String name : names) {
            DirectoryPlayer player = new DirectoryPlayer();
            player.name = name;
            player.level = levelOf(found, name);
            if (online.test(name)) {
                String status = Presence.statusOf(name);
                boolean self = viewer != null && viewer.equalsIgnoreCase(name);
                if (!Presence.HIDDEN.equals(status) || staff || self) {
                    player.online = true;
                    player.status = status;
                }
            }
            page.players.add(player);
        }
        return page;
    }

    private static Integer levelOf(Map<String, Integer> levels, String name) {
        Integer level = levels.get(name);
        if (level != null) {
            return level;
        }
        for (Map.Entry<String, Integer> entry : levels.entrySet()) {
            if (entry.getKey().equalsIgnoreCase(name)) {
                return entry.getValue();
            }
        }
        return null;
    }
}

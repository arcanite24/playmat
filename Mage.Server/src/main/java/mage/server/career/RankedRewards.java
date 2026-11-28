package mage.server.career;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;

/**
 * What first reaching a ranked rank pays (#431): the rank's title, sleeve and frame (the catalog's items with that
 * `ladder`, once per account) and, at Gold, Diamond and Mythic, a capsule token (once per format and queue; the
 * ladder keeps track of that). Paid into the Career profile, made now if the player has none.
 */
public final class RankedRewards {

    /** the ranks that also pay a capsule token */
    static final List<String> TOKEN_RANKS = java.util.Arrays.asList("gold", "diamond", "mythic");

    private RankedRewards() {
    }

    /** What reaching the rank gave: the cosmetics new to the account and the tokens */
    public static class Paid {
        public List<CareerProgress.CareerCosmetic> cosmetics = new ArrayList<>();
        public int tokens;
        /** League pack tokens of the format */
        public int packs;
        /** League wildcards by rarity */
        public java.util.Map<String, Integer> wildcards = new java.util.LinkedHashMap<>();
    }

    /** pays reaching `rank` (bronze to mythic) in a format for the first time */
    public static Paid pay(String user, String rank, String format, long now) throws SQLException {
        CareerStore store = CareerStore.get();
        Paid paid = new Paid();
        CareerService.ensureProfile(store, user, now);
        for (CareerCosmetics.CareerCatalogItem item : CareerCosmetics.get().items()) {
            if (rank.equals(item.ladder) && store.addUnlock(user, item.kind, item.id, now)) {
                paid.cosmetics.add(new CareerProgress.CareerCosmetic(item.kind, item.id));
            }
        }
        if (TOKEN_RANKS.contains(rank)) {
            CareerCapsules.addTokens(store, user, 1, "ranked " + format + " " + rank, now);
            paid.tokens = 1;
        }
        return paid;
    }
}

package mage.server.career;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

/**
 * Companion & pet bond evolution (Phase 6): every counted game with an equipped pet awards Bond XP, leveling the
 * companion from Tier 1 to Tier 5 and unlocking reactive table emotes (Tier 2), 3D/companion accessories (Tiers 3-4),
 * Astral Aura (Tier 5), and the Mythic Bond title at Tier 5.
 */
public final class CareerPets {

    /** XP required to reach each Bond Tier (index 0 = Tier 1, index 4 = Tier 5). */
    public static final int[] TIER_XP = {0, 100, 250, 600, 1200};
    public static final int MAX_TIER = TIER_XP.length;
    public static final String MYTHIC_BOND_TITLE = "mythic-bond";

    /** Accessory / aura styles mapped to the minimum Bond Tier required to equip them. */
    private static final Map<String, Integer> STYLE_TIERS;

    static {
        Map<String, Integer> map = new LinkedHashMap<>();
        map.put("crown", 3);
        map.put("shades", 3);
        map.put("wizard-hat", 4);
        map.put("mana-collar", 4);
        map.put("astral-aura", 5);
        STYLE_TIERS = Collections.unmodifiableMap(map);
    }

    /** One pet's bond progress, as returned to the client locker. */
    public static class CareerPetProgress {
        public String id;
        public int xp;
        public int tier;
        /** XP required for the next tier, or 0 at Tier 5 */
        public int nextTierXp;
        public int games;
        public int wins;
        /** currently equipped accessory/aura style id, or null for default */
        public String style;
        /** accessory/aura style ids unlocked at the pet's current tier */
        public List<String> unlockedStyles = new ArrayList<>();
    }

    /** Bond XP awarded to the active pet at the end of a game, shown on the rewards beat. */
    public static class CareerPetBondResult {
        public String petId;
        public int xpGained;
        public int xp;
        public int tier;
        public int nextTierXp;
        public boolean tieredUp;
        /** title id unlocked when crossing into Tier 5, or null */
        public String unlockedTitle;
    }

    private static volatile CareerPets instance;

    private CareerPets() {
    }

    public static CareerPets get() {
        CareerPets current = instance;
        if (current == null) {
            synchronized (CareerPets.class) {
                current = instance;
                if (current == null) {
                    current = new CareerPets();
                    instance = current;
                }
            }
        }
        return current;
    }

    public static int tierForXp(int xp) {
        int tier = 1;
        for (int i = 1; i < TIER_XP.length; i++) {
            if (xp >= TIER_XP[i]) {
                tier = i + 1;
            }
        }
        return tier;
    }

    public static int nextTierXp(int tier) {
        return tier >= MAX_TIER ? 0 : TIER_XP[Math.max(1, tier)];
    }

    public static List<String> unlockedStyles(int tier) {
        List<String> styles = new ArrayList<>();
        for (Map.Entry<String, Integer> entry : STYLE_TIERS.entrySet()) {
            if (tier >= entry.getValue()) {
                styles.add(entry.getKey());
            }
        }
        return styles;
    }

    public static boolean validStyleForTier(String style, int tier) {
        if (style == null || style.isEmpty() || "none".equals(style)) {
            return true;
        }
        Integer required = STYLE_TIERS.get(style);
        return required != null && tier >= required;
    }

    /** Every pet this account has bonded with, highest XP first. */
    public List<CareerPetProgress> list(String user) throws SQLException {
        List<CareerPetProgress> out = new ArrayList<>();
        for (CareerStore.PetProgressRow row : CareerStore.get().petProgress(user)) {
            out.add(toView(row));
        }
        return out;
    }

    /** One pet's bond row, or null before any games with it. */
    public CareerStore.PetProgressRow bondOf(String user, String petId) throws SQLException {
        String id = cleanId(petId);
        if (id == null) {
            return null;
        }
        return CareerStore.get().petProgress(user, id);
    }

    /** Equips or clears an accessory/aura style on a pet if its current Bond Tier unlocks it. */
    public CareerPetProgress setStyle(String user, String petId, String rawStyle) throws SQLException, CareerService.CareerException {
        String id = cleanId(petId);
        if (id == null) {
            throw new CareerService.CareerException("Pick a pet first.");
        }
        String parsed = cleanId(rawStyle);
        final String chosenStyle = "none".equals(parsed) ? null : parsed;
        CareerStore store = CareerStore.get();
        try {
            return store.transaction(() -> {
                CareerStore.PetProgressRow row = store.petProgress(user, id);
                if (row == null) {
                    row = new CareerStore.PetProgressRow();
                    row.petId = id;
                    row.tier = 1;
                }
                if (!validStyleForTier(chosenStyle, row.tier)) {
                    Integer needed = STYLE_TIERS.get(chosenStyle);
                    throw new SQLException(new CareerService.CareerException(needed == null
                            ? "Unknown companion style."
                            : "Reach Bond Tier " + needed + " with this companion to equip that style."));
                }
                row.style = chosenStyle;
                row.updated = System.currentTimeMillis();
                store.upsertPetProgress(user, row);
                return toView(row);
            });
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerService.CareerException) {
                throw (CareerService.CareerException) e.getCause();
            }
            throw e;
        }
    }

    /**
     * Awards Bond XP to the player's equipped pet after a counted game. Safe to call after the main payout transaction
     * so payout parity snapshots remain unchanged.
     */
    public CareerPetBondResult recordGame(CareerStore store, String user, String petId, boolean won,
                                          boolean commanderCast, boolean prestigeMode, long now) throws SQLException {
        String id = cleanId(petId);
        if (id == null) {
            return null;
        }
        int xpDelta = 10 + (won ? 10 : 0) + (commanderCast ? 5 : 0) + (prestigeMode && won ? 10 : 0);
        return store.transaction(() -> {
            CareerStore.PetProgressRow row = store.petProgress(user, id);
            int tierBefore = row == null ? 1 : Math.max(1, row.tier);
            if (row == null) {
                row = new CareerStore.PetProgressRow();
                row.petId = id;
                row.tier = 1;
            }
            row.xp += xpDelta;
            row.tier = tierForXp(row.xp);
            row.games += 1;
            if (won) {
                row.wins += 1;
            }
            row.updated = now;
            store.upsertPetProgress(user, row);

            CareerPetBondResult bond = new CareerPetBondResult();
            bond.petId = id;
            bond.xpGained = xpDelta;
            bond.xp = row.xp;
            bond.tier = row.tier;
            bond.nextTierXp = nextTierXp(row.tier);
            bond.tieredUp = row.tier > tierBefore;
            if (row.tier >= MAX_TIER && tierBefore < MAX_TIER) {
                store.addUnlock(user, "title", MYTHIC_BOND_TITLE, now);
                bond.unlockedTitle = MYTHIC_BOND_TITLE;
            }
            return bond;
        });
    }

    private static CareerPetProgress toView(CareerStore.PetProgressRow row) {
        CareerPetProgress view = new CareerPetProgress();
        view.id = row.petId;
        view.xp = row.xp;
        view.tier = Math.max(1, row.tier);
        view.nextTierXp = nextTierXp(view.tier);
        view.games = row.games;
        view.wins = row.wins;
        view.style = validStyleForTier(row.style, view.tier) ? row.style : null;
        view.unlockedStyles = unlockedStyles(view.tier);
        return view;
    }

    private static String cleanId(String raw) {
        if (raw == null) {
            return null;
        }
        String text = raw.trim().toLowerCase(Locale.ROOT);
        return text.isEmpty() || text.length() > 40 || !text.matches("[a-z0-9-]+") ? null : text;
    }
}

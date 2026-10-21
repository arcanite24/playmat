package mage.server.support;

import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/**
 * What supporting the server gives: a thank-you set of cosmetics, all drawn in Playmat's own style (CSS, SVG and
 * procedural art; never Wizards of the Coast's). Support never unlocks gameplay, formats, cards, queues or Career
 * content, so every kind here is cosmetic, and a test keeps it that way. The client knows how each one looks.
 */
public final class SupportPerks {

    /** the only kinds a perk may be: what a player looks like, never what they can do */
    public static final Set<String> COSMETIC_KINDS = Collections.unmodifiableSet(new LinkedHashSet<>(Arrays.asList(
            "sleeve", "playmat", "avatar", "frame", "title", "flair", "pet")));

    /** One perk: its kind and item. */
    public static final class Perk {
        public final String kind;
        public final String item;

        Perk(String kind, String item) {
            this.kind = kind;
            this.item = item;
        }
    }

    public static final String SLEEVE = "supporter";
    public static final String PLAYMAT = "patron";
    public static final String FRAME = "supporter-ring";
    public static final String TITLE = "patron";
    public static final String FLAIR = "heart";
    /** a companion on the battlefield, a little 3D model that plays when clicked */
    public static final String PET = "party-corgi";

    /** the permanent thank-you set any linked payment unlocks */
    public static final List<Perk> THANK_YOU = Collections.unmodifiableList(Arrays.asList(
            new Perk("sleeve", SLEEVE),
            new Perk("playmat", PLAYMAT),
            new Perk("frame", FRAME),
            new Perk("title", TITLE),
            new Perk("flair", FLAIR),
            new Perk("pet", PET)
    ));

    private SupportPerks() {
    }

    public static boolean isPerk(String kind, String item) {
        for (Perk perk : THANK_YOU) {
            if (perk.kind.equals(kind) && perk.item.equals(item)) {
                return true;
            }
        }
        return false;
    }
}

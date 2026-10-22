package mage.server.support;

/**
 * The look a player plays with, as their opponents see it: the sleeve on their cards, their playmat cloth, a Career
 * avatar, the supporter ring around it, a title under their name, a flair glyph beside it and a pet on their side of the
 * battlefield. Every field is optional;
 * the server keeps only what the account owns (see {@link Support#validLook}).
 */
public class PlayerLook {

    /** the account's name (in answers) */
    public String name;
    /** a free sleeve's colour (#rrggbb), a Career sleeve's id, or supporter */
    public String sleeve;
    /** a playmat cloth's id */
    public String cloth;
    /** a Career avatar's id */
    public String avatar;
    /** supporter-ring */
    public String frame;
    /** a Career title's id, or patron */
    public String title;
    /** heart */
    public String flair;
    /** a pet's id: party-corgi, or an earned companion */
    public String pet;
    /** an attack arrow colour's id (B300) */
    public String arrow;
    /** a hit spark's id (B300) */
    public String spark;
    /** a victory stamp's id (B300) */
    public String stamp;
    /** a basic land style's id: which full-art printing the basics show (B301) */
    public String lands;
    /** up to three achievement badges shown on the plate and profile (B307) */
    public java.util.List<String> badges;
    /** the full-art card styles the player wears, card name to style (M30); filled by the server from the capsule shop */
    public java.util.Map<String, String> cardStyles;
    /** the account's progression level (B279); null on servers without Career or before its first counted game */
    public Integer level;

    public PlayerLook() {
    }

    boolean isEmpty() {
        return sleeve == null && cloth == null && avatar == null && frame == null && title == null && flair == null && pet == null
                && arrow == null && spark == null && stamp == null && lands == null && (badges == null || badges.isEmpty())
                && (cardStyles == null || cardStyles.isEmpty());
    }
}

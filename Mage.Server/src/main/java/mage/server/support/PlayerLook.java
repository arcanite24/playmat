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
    /** a pet's id: party-corgi */
    public String pet;
    /** the account's progression level (B279); null on servers without Career or before its first counted game */
    public Integer level;

    public PlayerLook() {
    }

    boolean isEmpty() {
        return sleeve == null && cloth == null && avatar == null && frame == null && title == null && flair == null && pet == null;
    }
}

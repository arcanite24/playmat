package mage.server.game;

import java.io.Serializable;

/**
 * A quick emote sent in a game (B298): who sent it and which sticker or phrase. Every player and watcher of the game
 * gets it; muting an opponent's emotes happens on the client.
 */
public class PlayerEmote implements Serializable {

    public String gameId;
    /** the player's name */
    public String player;
    /** a sticker or phrase id from an emote set (career/cosmetics.json) */
    public String emote;

    public PlayerEmote() {
    }

    PlayerEmote(String gameId, String player, String emote) {
        this.gameId = gameId;
        this.player = player;
        this.emote = emote;
    }
}

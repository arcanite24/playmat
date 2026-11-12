package mage.server.decks;

import com.j256.ormlite.field.DatabaseField;
import com.j256.ormlite.table.DatabaseTable;

/**
 * A deck its owner shares with everyone on the server (#137). The deck itself stays in web_deck: a shared deck is
 * always the owner's latest synced version, and a deleted one drops out of the shared list.
 */
@DatabaseTable(tableName = "web_deck_share")
public class StoredDeckShare {

    /** owner + '/' + deckId, as in web_deck */
    @DatabaseField(id = true)
    String key;
    /** account name, lower case */
    @DatabaseField(index = true)
    String owner;
    /** the account name as its owner writes it, for the shared list */
    @DatabaseField
    String ownerName;
    @DatabaseField
    String deckId;
    /** server time it was shared, milliseconds */
    @DatabaseField
    long sharedAt;

    public StoredDeckShare() {
    }
}

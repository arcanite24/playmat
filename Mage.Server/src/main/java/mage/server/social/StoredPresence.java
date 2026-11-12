package mage.server.social;

import com.j256.ormlite.field.DatabaseField;
import com.j256.ormlite.table.DatabaseTable;

/** The status one account chose (only accounts that changed it from online have a row). */
@DatabaseTable(tableName = "web_presence")
public class StoredPresence {

    /** account name, lower case */
    @DatabaseField(id = true)
    String account;
    /** {@link Presence#HIDDEN} or {@link Presence#LOOKING} */
    @DatabaseField
    String status;
    @DatabaseField
    long updatedAt;

    public StoredPresence() {
    }
}

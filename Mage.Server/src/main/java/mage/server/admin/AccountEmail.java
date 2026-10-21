package mage.server.admin;

import com.j256.ormlite.field.DatabaseField;
import com.j256.ormlite.table.DatabaseTable;

/**
 * What the server knows about an account's email beyond the address: the address its player proved they read (with
 * a code sent to it), and when the account was registered. The account counts as verified only while its email is
 * still that address.
 */
@DatabaseTable(tableName = "account_email")
public class AccountEmail {

    /** the account name in lower case */
    @DatabaseField(id = true)
    public String key;
    @DatabaseField
    public String name;
    /** the verified address, or null */
    @DatabaseField
    public String verifiedEmail;
    @DatabaseField
    public long verifiedAt;
    /** 0 for accounts registered before the server kept it */
    @DatabaseField
    public long registeredAt;

    public AccountEmail() {
    }
}

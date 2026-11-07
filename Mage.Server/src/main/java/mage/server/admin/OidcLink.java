package mage.server.admin;

import com.j256.ormlite.field.DatabaseField;
import com.j256.ormlite.table.DatabaseTable;

/**
 * An account that signs in with the identity provider: the provider's stable user id (it never changes)
 * and its display details when last seen, plus the last verified ID token: the end-session's id_token_hint
 * for sign-outs from sessions that hold no token of their own (a reload, a device). A sign-in is the only
 * thing that refreshes it, so it goes stale - and that is safe: the provider ends the session by the hint's
 * user, and a hint whose session is already gone ends nothing and bounces the browser back.
 */
@DatabaseTable(tableName = "oidc_link")
public class OidcLink {

    @DatabaseField(id = true)
    public String sub;
    @DatabaseField(index = true)
    public String displayName;
    @DatabaseField
    public String email;
    @DatabaseField(index = true)
    public String account;
    @DatabaseField
    public long linkedAt;
    @DatabaseField
    public String lastToken;
    @DatabaseField
    public long lastTokenExp;

    public OidcLink() {
    }
}

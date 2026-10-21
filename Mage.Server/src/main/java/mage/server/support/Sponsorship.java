package mage.server.support;

/**
 * A recurring GitHub sponsorship. Each month it is active it adds a ledger payment of its monthly amount, so the month
 * totals, the supporters count and staff's ledger treat it like any other payment.
 */
public class Sponsorship {

    /** the sponsor's GitHub user id (it survives renames) */
    public long githubId;
    public String login;
    /** GitHub's node id of the sponsorship: the ledger's payments are keyed by it */
    public String nodeId;
    public long monthlyCents;
    public String tier;
    /** the sponsor's privacy level is public */
    public boolean isPublic;
    /** cancelled sponsorships stay in the table, inactive */
    public boolean active;
    public long since;
    /** when it ends (a pending cancellation) or ended; 0 while it runs on */
    public long until;
    /** the account the sponsor signs in to with GitHub, or null until they do */
    public String account;

    public Sponsorship() {
    }

    /** whether it pays for the moment {@code now} */
    boolean runs(long now) {
        return active && (until == 0 || until > now);
    }
}

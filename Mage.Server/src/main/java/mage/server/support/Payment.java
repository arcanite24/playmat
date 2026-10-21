package mage.server.support;

/**
 * One support payment in the ledger: a Ko-fi donation, membership payment, shop order or commission, or a month of a
 * GitHub sponsorship. The payer's email is never kept, only its peppered hash (to follow renewals to the account the
 * first payment linked to); the provider's raw payload is kept 90 days for staff, then dropped. Both stay on the
 * server: they are transient, so no view or export carries them.
 */
public class Payment {

    public long id;
    /** kofi or github */
    public String provider;
    /** Ko-fi's transaction id, or the sponsorship's node id and the month it pays for */
    public String providerId;
    /** donation, membership, shop, commission or sponsorship */
    public String kind;
    public long amountCents;
    /** as the provider sent it (Ko-fi sends the page's currency); only USD counts toward the goal */
    public String currency;
    /** the membership or sponsorship tier, if any */
    public String tier;
    /** the payer's display name at the provider */
    public String fromName;
    /** what the payer wrote, trimmed to {@link Support#MAX_MESSAGE} characters */
    public String message;
    /** the payer let the provider show the payment publicly */
    public boolean isPublic;
    /** the account it is linked to, or null while unmatched */
    public String account;
    /** auto or staff; null while unmatched */
    public String linkedBy;
    public long receivedAt;
    /** staff revoked it (a refund or a chargeback): it no longer counts, and its perks may have been taken back */
    public boolean revoked;
    public String revokedReason;
    /** the GitHub sponsor's id, for a sponsorship's months */
    public long sponsorId;

    transient String emailHash;
    transient String rawPayload;

    public Payment() {
    }
}

package mage.server.support;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParseException;
import com.google.gson.JsonParser;
import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.EmailVerification;
import mage.server.admin.AccessPolicy;
import mage.server.admin.GithubLink;
import mage.server.admin.RuntimeSettings;
import mage.server.career.CareerStore;
import mage.server.notify.DiscordNotifier;
import org.apache.log4j.Logger;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;
import java.sql.SQLException;
import java.time.Instant;
import java.time.YearMonth;
import java.time.ZoneOffset;
import java.time.format.DateTimeParseException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Players keep the lights on: support payments arrive from Ko-fi and GitHub Sponsors (see {@link SupportHooks}), go in
 * the ledger, link to an account, unlock the thank-you perks and count toward the month's goal, all within the request
 * that delivered them. Staff only handle payments nothing could link.
 * <p>
 * The rules: the game is free forever and support never unlocks gameplay; perks are cosmetic (see {@link SupportPerks})
 * and are never taken back, except when staff revoke a refunded payment. A payment links to an account by, in order:
 * the account's support code in the payment's message; the payment's email, when it is the account's confirmed email
 * (an unconfirmed one never links); an earlier linked payment from the same email (renewals follow the first). The
 * goal counts the current calendar month (UTC), in US dollars.
 */
public final class Support {

    private static final Logger logger = Logger.getLogger(Support.class);

    public static final int MAX_MESSAGE = 500;
    static final int MAX_NAME = 100;
    static final long DAY = TimeUnit.DAYS.toMillis(1);
    /** providers' raw payloads are kept this long, for staff, then dropped */
    static final long PAYLOAD_MILLIS = 90 * DAY;
    /** a payment makes its payer a supporter for this long */
    static final long SUPPORTER_MILLIS = 30 * DAY;
    /** a Ko-fi membership counts as running a few days past its month, while the renewal arrives */
    static final long MEMBERSHIP_MILLIS = 35 * DAY;
    static final int WALL_MAX = 30;
    static final String CODE_PREFIX = "PM-";
    /** no 0/O, 1/I/L: a code read aloud or retyped from a phone can't be mistaken */
    static final String CODE_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    static final Pattern CODE_IN_TEXT = Pattern.compile("(?i)\\bPM-?([A-Z0-9]{4,6})\\b");
    /** how long the public status may be served from memory */
    static final long STATUS_CACHE_MILLIS = 15_000;

    /** The free sleeves (Mage.Web.Client stores/decks.ts SLEEVE_COLORS): anyone may play with them. */
    static final Set<String> FREE_SLEEVES = Collections.unmodifiableSet(new HashSet<>(Arrays.asList(
            "#20302b", "#1f2a44", "#3b1f2b", "#4a2a17", "#2c2c2e", "#3e3a2a", "#24403f", "#5a2320")));
    /** The free playmat cloths (Mage.Web.Client match/playmats.ts MAT_CLOTHS without career or perk). */
    static final Set<String> FREE_CLOTHS = Collections.unmodifiableSet(new HashSet<>(Arrays.asList(
            "green", "navy", "plum", "oxblood", "teal", "charcoal")));

    /** Who the accounts are, as support needs to know them: the server's, or a stand-in in tests. */
    public interface Accounts {
        /** the account's name as registered, or null when there is no such account */
        String name(String userName);

        /** the account whose email is this address and is confirmed, or null */
        String confirmedEmailOwner(String email);

        /** the account a GitHub user signs in to, or null */
        String githubAccount(long githubId);

        /** the account's Career cosmetics, as {kind, item} */
        List<String[]> careerUnlocks(String account);

        /** the achievement and mastery badges the account may show */
        default Set<String> careerBadges(String account) {
            return Collections.emptySet();
        }

        /** the card styles the account wears and owns: card name to style (M30) */
        default java.util.Map<String, String> careerCardStyles(String account) {
            return Collections.emptyMap();
        }
    }

    /** The public thank-you post (Discord). */
    public interface Thanks {
        /** @param amount the amount as text, or null when the owner keeps amounts private */
        void supported(String name, String amount);
    }

    /** What a delivery did to the ledger. */
    public static final class Recorded {
        public final Payment payment;
        /** the provider sent this payment before: nothing changed */
        public final boolean duplicate;

        Recorded(Payment payment, boolean duplicate) {
            this.payment = payment;
            this.duplicate = duplicate;
        }
    }

    // ---- views

    /** One perk a player owns. */
    public static class SupportPerk {
        /** sleeve, playmat, frame, title or flair */
        public String kind;
        public String item;
        public long grantedAt;
    }

    /** The signed-in player's side of support. */
    public static class SupportYou {
        /** the code to put in a Ko-fi message, like PM-7Q4K */
        public String code;
        /** a membership or sponsorship runs, or a payment arrived in the last 30 days */
        public boolean supporter;
        /** the first payment (or staff's grant), 0 before any */
        public long since;
        /** owned for good */
        public List<SupportPerk> perks;
        /** the player chose to appear on the supporters wall */
        public boolean wall;
        /** a payment linked since the player last looked: show the thank-you */
        public boolean notice;
    }

    /** The month's goal and who helped, for everyone. */
    public static class SupportStatus {
        /** the support UI is on and has a page to send players to */
        public boolean enabled;
        public String kofiUrl;
        public String sponsorsUrl;
        public long goalCents;
        /** in US cents, this calendar month (UTC) */
        public long monthCents;
        /** the month, as 2026-10 */
        public String monthLabel;
        /** distinct payers this month */
        public int supporters;
        /** what the money pays for, a line each */
        public List<String> costs;
        /** this month's public names, newest first, 30 at most */
        public List<String> publicSupporters;
        /** the signed-in player, when there is one */
        public SupportYou you;
    }

    /** Staff's view of a month of the ledger. */
    public static class SupportLedger {
        public String month;
        public List<Payment> payments;
        /** in US cents, revoked payments left out */
        public long monthCents;
        public long goalCents;
        public int supporters;
        /** payments in other currencies this month, which the goal doesn't count, like "EUR 5.00" */
        public List<String> otherCurrencies;
        /** payments no account is linked to, in every month */
        public long unmatched;
    }

    private static volatile Support instance;

    private final SupportStore store;
    private final Accounts accounts;
    private final Thanks thanks;
    private final SecureRandom random = new SecureRandom();
    private volatile SupportStatus cachedStatus;
    private volatile long cachedAt;

    public Support(SupportStore store, Accounts accounts, Thanks thanks) {
        this.store = store;
        this.accounts = accounts;
        this.thanks = thanks;
    }

    /** The server's support, on db/support.db. */
    public static Support get() {
        Support current = instance;
        if (current == null) {
            synchronized (Support.class) {
                current = instance;
                if (current == null) {
                    current = new Support(SupportStore.get(), serverAccounts(),
                            (name, amount) -> DiscordNotifier.instance().supported(name, amount));
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests: the support every server class sees (null goes back to the server's). */
    public static void use(Support support) {
        instance = support;
    }

    public SupportStore store() {
        return store;
    }

    // ---- Ko-fi

    /** Records one Ko-fi payment, links it and grants its perks; a delivery seen before changes nothing. */
    public synchronized Recorded recordKofi(KofiPayload kofi, long now) throws SQLException {
        Payment existing = store.paymentByProviderId(kofi.transactionId);
        if (existing != null) {
            return new Recorded(existing, true);
        }
        Payment payment = new Payment();
        payment.provider = "kofi";
        payment.providerId = kofi.transactionId;
        payment.kind = kofi.kind();
        payment.amountCents = kofi.amountCents;
        payment.currency = kofi.currency;
        payment.tier = clip(kofi.tierName, MAX_NAME);
        payment.fromName = clip(kofi.fromName, MAX_NAME);
        payment.message = clip(kofi.message, MAX_MESSAGE);
        payment.isPublic = kofi.isPublic;
        payment.emailHash = emailHash(kofi.email);
        payment.receivedAt = now;
        payment.rawPayload = withoutSecrets(kofi.json);
        payment.account = findAccount(kofi.message, kofi.email, payment.emailHash);
        payment.linkedBy = payment.account == null ? null : "auto";
        if (!store.addPayment(payment)) {
            return new Recorded(store.paymentByProviderId(kofi.transactionId), true);
        }
        if (payment.account != null) {
            granted(payment.account, payment.id, now, true);
        }
        logRecorded(payment);
        announce(payment);
        changed();
        return new Recorded(payment, false);
    }

    /**
     * The account a payment links to: a support code in its message, then its email when an account confirmed it,
     * then the account an earlier payment from the same email went to. Null: staff decide.
     */
    String findAccount(String message, String email, String emailHash) throws SQLException {
        if (message != null) {
            Matcher codes = CODE_IN_TEXT.matcher(message);
            while (codes.find()) {
                String owner = store.accountOfCode(CODE_PREFIX + codes.group(1).toUpperCase(Locale.ROOT));
                String name = owner == null ? null : accounts.name(owner);
                if (name != null) {
                    return name;
                }
            }
        }
        if (email != null && !email.trim().isEmpty()) {
            String owner = accounts.confirmedEmailOwner(email.trim());
            if (owner != null) {
                return owner;
            }
        }
        String earlier = store.accountOfEmailHash(emailHash);
        return earlier == null ? null : accounts.name(earlier);
    }

    /** SHA-256 of the server's pepper and the lowercased address; null without an address. */
    String emailHash(String email) throws SQLException {
        if (email == null || email.trim().isEmpty()) {
            return null;
        }
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            digest.update(store.pepper().getBytes(StandardCharsets.UTF_8));
            digest.update((byte) ':');
            digest.update(email.trim().toLowerCase(Locale.ROOT).getBytes(StandardCharsets.UTF_8));
            return GithubSponsorsEvent.hex(digest.digest());
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    /** The payload as staff may read it: without Ko-fi's verification token and the payer's email. */
    static String withoutSecrets(String json) {
        try {
            JsonElement parsed = JsonParser.parseString(json);
            if (parsed.isJsonObject()) {
                JsonObject object = parsed.getAsJsonObject();
                object.remove("verification_token");
                object.remove("email");
                return object.toString();
            }
        } catch (JsonParseException e) {
            // not ours to keep
        }
        return null;
    }

    // ---- GitHub Sponsors

    /** Applies one sponsorship event: a new, changed or ending sponsorship, or a one-time payment. */
    public synchronized void recordSponsorship(GithubSponsorsEvent event, long now) throws SQLException {
        Sponsorship sponsorship = store.sponsorship(event.sponsorId);
        String account = accounts.githubAccount(event.sponsorId);
        switch (event.action) {
            case "created":
                if (event.oneTime) {
                    Payment payment = new Payment();
                    payment.provider = "github";
                    payment.providerId = event.nodeId;
                    payment.kind = "donation";
                    payment.amountCents = event.monthlyCents;
                    payment.currency = "USD";
                    payment.tier = clip(event.tierName, MAX_NAME);
                    payment.fromName = clip(event.sponsorLogin, MAX_NAME);
                    payment.isPublic = event.isPublic;
                    payment.sponsorId = event.sponsorId;
                    payment.account = account;
                    payment.linkedBy = account == null ? null : "auto";
                    payment.receivedAt = now;
                    payment.rawPayload = event.json;
                    if (store.addPayment(payment)) {
                        if (account != null) {
                            granted(account, payment.id, now, true);
                        }
                        logRecorded(payment);
                        announce(payment);
                    }
                    break;
                }
                if (sponsorship == null) {
                    sponsorship = new Sponsorship();
                    sponsorship.githubId = event.sponsorId;
                }
                sponsorship.nodeId = event.nodeId;
                sponsorship.since = event.createdAt > 0 ? event.createdAt : now;
                sponsorship.until = 0;
                sponsorship.active = true;
                apply(sponsorship, event, account, now);
                break;
            case "tier_changed":
                if (sponsorship == null) {
                    sponsorship = new Sponsorship();
                    sponsorship.githubId = event.sponsorId;
                    sponsorship.nodeId = event.nodeId;
                    sponsorship.since = event.createdAt > 0 ? event.createdAt : now;
                    sponsorship.active = true;
                }
                apply(sponsorship, event, account, now);
                break;
            case "edited":
                if (sponsorship != null) {
                    apply(sponsorship, event, account, now);
                }
                break;
            case "pending_cancellation":
                if (sponsorship != null) {
                    sponsorship.until = event.effectiveAt > 0 ? event.effectiveAt : now;
                    store.putSponsorship(sponsorship, now);
                }
                break;
            case "cancelled":
                if (sponsorship != null) {
                    sponsorship.active = false;
                    sponsorship.until = now;
                    store.putSponsorship(sponsorship, now);
                }
                break;
            default:
                // pending_tier_change: tier_changed follows when it takes effect
                break;
        }
        logger.info("Support: GitHub sponsorship " + event.action);
        changed();
    }

    private void apply(Sponsorship sponsorship, GithubSponsorsEvent event, String account, long now) throws SQLException {
        sponsorship.login = event.sponsorLogin;
        if (event.monthlyCents > 0) {
            sponsorship.monthlyCents = event.monthlyCents;
        }
        sponsorship.tier = clip(event.tierName, MAX_NAME);
        sponsorship.isPublic = event.isPublic;
        if (sponsorship.account == null) {
            sponsorship.account = account;
        }
        store.putSponsorship(sponsorship, now);
        Payment month = ensurePeriod(sponsorship, now);
        if (month != null) {
            announce(month);
        }
    }

    /** The month's ledger payment of a running sponsorship, when it was missing (and is now added); otherwise null. */
    private Payment ensurePeriod(Sponsorship sponsorship, long now) throws SQLException {
        if (!sponsorship.runs(now) || sponsorship.monthlyCents <= 0 || sponsorship.nodeId == null) {
            return null;
        }
        YearMonth month = month(now);
        Payment payment = new Payment();
        payment.provider = "github";
        payment.providerId = sponsorship.nodeId + ":" + month;
        payment.kind = "sponsorship";
        payment.amountCents = sponsorship.monthlyCents;
        payment.currency = "USD";
        payment.tier = sponsorship.tier;
        payment.fromName = sponsorship.login;
        payment.isPublic = sponsorship.isPublic;
        payment.sponsorId = sponsorship.githubId;
        payment.account = sponsorship.account;
        payment.linkedBy = sponsorship.account == null ? null : "auto";
        payment.receivedAt = Math.max(sponsorship.since, start(month));
        if (!store.addPayment(payment)) {
            return null;
        }
        if (payment.account != null) {
            granted(payment.account, payment.id, now, true);
        }
        logRecorded(payment);
        return payment;
    }

    /** Each running sponsorship's payment for this month; ended ones stop. */
    synchronized void ensurePeriods(long now) throws SQLException {
        for (Sponsorship sponsorship : store.sponsorships()) {
            if (sponsorship.active && sponsorship.until > 0 && sponsorship.until <= now) {
                sponsorship.active = false;
                store.putSponsorship(sponsorship, now);
                continue;
            }
            ensurePeriod(sponsorship, now);
        }
    }

    /** A GitHub user signed in to this account: their sponsorship and its months link to it. */
    public synchronized void githubLinked(long githubId, String account, long now) throws SQLException {
        Sponsorship sponsorship = store.sponsorship(githubId);
        if (sponsorship != null && sponsorship.account == null) {
            sponsorship.account = account;
            store.putSponsorship(sponsorship, now);
        }
        List<Long> linked = store.linkSponsorPayments(githubId, account);
        for (long id : linked) {
            granted(account, id, now, true);
        }
        if (!linked.isEmpty()) {
            logger.info("Support: " + linked.size() + " GitHub payment(s) linked at sign-in");
            changed();
        }
    }

    // ---- perks

    /** Grants the thank-you set for a payment (0: staff's grant); a thank-you waits for the next visit when anything was new. */
    private void granted(String account, long paymentId, long now, boolean notify) throws SQLException {
        boolean any = false;
        for (SupportPerks.Perk perk : SupportPerks.THANK_YOU) {
            any |= store.addPerk(account, perk.kind, perk.item, now, paymentId);
        }
        if (any && notify) {
            store.setNotice(account, paymentId, now);
        }
    }

    public synchronized SupportYou you(String account, long now) throws SQLException {
        SupportYou you = new SupportYou();
        you.code = codeFor(account, now);
        you.perks = new ArrayList<>();
        long since = 0;
        for (SupportStore.OwnedPerk owned : store.perks(account)) {
            SupportPerk perk = new SupportPerk();
            perk.kind = owned.kind;
            perk.item = owned.item;
            perk.grantedAt = owned.grantedAt;
            you.perks.add(perk);
            since = since == 0 ? owned.grantedAt : Math.min(since, owned.grantedAt);
        }
        for (Payment payment : store.paymentsOf(account)) {
            if (payment.revoked) {
                continue;
            }
            since = since == 0 ? payment.receivedAt : Math.min(since, payment.receivedAt);
            long age = now - payment.receivedAt;
            if (age < SUPPORTER_MILLIS || ("membership".equals(payment.kind) && age < MEMBERSHIP_MILLIS)) {
                you.supporter = true;
            }
        }
        for (Sponsorship sponsorship : store.sponsorshipsOf(account)) {
            you.supporter |= sponsorship.runs(now);
        }
        you.since = since;
        you.wall = store.onWall(account);
        you.notice = store.hasNotice(account);
        return you;
    }

    /** The account's support code, made the first time it is asked for. */
    public synchronized String codeFor(String account, long now) throws SQLException {
        String code = store.codeOf(account);
        for (int tries = 0; code == null && tries < 60; tries++) {
            int length = 4 + tries / 20;
            StringBuilder candidate = new StringBuilder(CODE_PREFIX);
            for (int i = 0; i < length; i++) {
                candidate.append(CODE_ALPHABET.charAt(random.nextInt(CODE_ALPHABET.length())));
            }
            if (store.addCode(account, candidate.toString(), now)) {
                code = candidate.toString();
            }
        }
        return code;
    }

    /** @return whether a thank-you was waiting */
    public boolean noticeSeen(String account) throws SQLException {
        return store.clearNotice(account);
    }

    public void setWall(String account, boolean wall) throws SQLException {
        store.setWall(account, wall);
        changed();
    }

    // ---- looks

    /** Keeps the look an account plays with, as far as it owns it; returns what was kept. */
    public PlayerLook setLook(String account, PlayerLook wanted, long now) throws SQLException {
        PlayerLook look = validLook(account, wanted);
        store.putLook(account, look, now);
        look.name = account;
        return look;
    }

    /** The looks of these players that have one, each still checked against what they own. */
    public List<PlayerLook> looks(List<String> names) throws SQLException {
        List<PlayerLook> looks = new ArrayList<>();
        Set<String> seen = new HashSet<>();
        for (String name : names) {
            if (name == null || !seen.add(name.toLowerCase(Locale.ROOT)) || seen.size() > 16) {
                continue;
            }
            PlayerLook stored = store.look(name.trim());
            if (stored == null) {
                continue;
            }
            PlayerLook look = validLook(stored.name, stored);
            if (!look.isEmpty()) {
                look.name = stored.name;
                looks.add(look);
            }
        }
        return looks;
    }

    /**
     * What of the look the account may wear: free sleeves and cloths, its Career cosmetics and its perks. Anything
     * else is dropped.
     */
    public PlayerLook validLook(String account, PlayerLook wanted) throws SQLException {
        Set<String> owned = new HashSet<>();
        for (SupportStore.OwnedPerk perk : store.perks(account)) {
            owned.add(perk.kind + ':' + perk.item);
        }
        for (String[] unlock : accounts.careerUnlocks(account)) {
            owned.add(unlock[0] + ':' + unlock[1]);
        }
        PlayerLook look = new PlayerLook();
        if (wanted == null) {
            return look;
        }
        String sleeve = clean(wanted.sleeve);
        look.sleeve = sleeve != null && (FREE_SLEEVES.contains(sleeve) || owned.contains("sleeve:" + sleeve)) ? sleeve : null;
        String cloth = clean(wanted.cloth);
        look.cloth = cloth != null && (FREE_CLOTHS.contains(cloth) || owned.contains("playmat:" + cloth)) ? cloth : null;
        look.avatar = owned(owned, "avatar", wanted.avatar);
        look.frame = owned(owned, "frame", wanted.frame);
        look.title = owned(owned, "title", wanted.title);
        look.flair = owned(owned, "flair", wanted.flair);
        look.pet = owned(owned, "pet", wanted.pet);
        look.arrow = owned(owned, "arrow", wanted.arrow);
        look.spark = owned(owned, "spark", wanted.spark);
        look.stamp = owned(owned, "stamp", wanted.stamp);
        look.lands = owned(owned, "land", wanted.lands);
        if (wanted.badges != null) {
            Set<String> earned = accounts.careerBadges(account);
            List<String> badges = new ArrayList<>();
            for (String badge : wanted.badges) {
                String id = clean(badge);
                if (id != null && earned.contains(id) && !badges.contains(id) && badges.size() < 3) {
                    badges.add(id);
                }
            }
            look.badges = badges.isEmpty() ? null : badges;
        }
        // card styles are worn from the capsule shop, not sent with the look: the server says which (M30)
        java.util.Map<String, String> styles = accounts.careerCardStyles(account);
        look.cardStyles = styles.isEmpty() ? null : new java.util.TreeMap<>(styles);
        return look;
    }

    private static String owned(Set<String> owned, String kind, String item) {
        String id = clean(item);
        return id != null && owned.contains(kind + ':' + id) ? id : null;
    }

    private static String clean(String id) {
        if (id == null) {
            return null;
        }
        String text = id.trim().toLowerCase(Locale.ROOT);
        return text.isEmpty() || text.length() > 40 || !text.matches("[a-z0-9#-]+") ? null : text;
    }

    // ---- the month

    /** The month's goal and supporters, for everyone; with the signed-in player's side when {@code account} is set. */
    public SupportStatus status(String account, long now) throws SQLException {
        SupportStatus status = publicStatus(now);
        if (account == null) {
            return status;
        }
        SupportStatus mine = copy(status);
        mine.you = you(account, now);
        return mine;
    }

    private SupportStatus publicStatus(long now) throws SQLException {
        SupportStatus cached = cachedStatus;
        if (cached != null && now - cachedAt >= 0 && now - cachedAt < STATUS_CACHE_MILLIS && cached.monthLabel.equals(month(now).toString())) {
            return cached;
        }
        ensurePeriods(now);
        RuntimeSettings settings = RuntimeSettings.get();
        YearMonth month = month(now);
        List<Payment> payments = store.paymentsBetween(start(month), start(month.plusMonths(1)));
        SupportStatus status = new SupportStatus();
        status.kofiUrl = settings.supportKofiUrl();
        status.sponsorsUrl = settings.supportSponsorsUrl();
        status.enabled = settings.supportOn() && !(status.kofiUrl.isEmpty() && status.sponsorsUrl.isEmpty());
        status.goalCents = settings.supportGoalCents();
        status.monthLabel = month.toString();
        status.monthCents = usdCents(payments);
        status.supporters = supporters(payments);
        status.costs = costs(settings.supportCosts());
        status.publicSupporters = wall(payments);
        cachedStatus = status;
        cachedAt = now;
        return status;
    }

    private static SupportStatus copy(SupportStatus status) {
        SupportStatus copy = new SupportStatus();
        copy.enabled = status.enabled;
        copy.kofiUrl = status.kofiUrl;
        copy.sponsorsUrl = status.sponsorsUrl;
        copy.goalCents = status.goalCents;
        copy.monthCents = status.monthCents;
        copy.monthLabel = status.monthLabel;
        copy.supporters = status.supporters;
        copy.costs = status.costs;
        copy.publicSupporters = status.publicSupporters;
        return copy;
    }

    private void changed() {
        cachedStatus = null;
    }

    static long usdCents(List<Payment> payments) {
        long cents = 0;
        for (Payment payment : payments) {
            if (!payment.revoked && "USD".equals(payment.currency)) {
                cents += payment.amountCents;
            }
        }
        return cents;
    }

    static int supporters(List<Payment> payments) {
        Set<String> payers = new HashSet<>();
        for (Payment payment : payments) {
            if (payment.revoked) {
                continue;
            }
            if (payment.account != null) {
                payers.add("a:" + payment.account.toLowerCase(Locale.ROOT));
            } else if (payment.sponsorId > 0) {
                payers.add("g:" + payment.sponsorId);
            } else if (payment.emailHash != null) {
                payers.add("e:" + payment.emailHash);
            } else {
                payers.add("p:" + payment.id);
            }
        }
        return payers.size();
    }

    /**
     * The supporters wall, newest first: a linked payment shows its account's name when the player chose to appear;
     * an unlinked one shows its display name when the payer made it public at the provider.
     */
    List<String> wall(List<Payment> newestFirst) throws SQLException {
        Set<String> seen = new HashSet<>();
        List<String> names = new ArrayList<>();
        for (Payment payment : newestFirst) {
            String name = publicName(payment);
            if (name != null && seen.add(name.toLowerCase(Locale.ROOT))) {
                names.add(name);
                if (names.size() >= WALL_MAX) {
                    break;
                }
            }
        }
        return names;
    }

    /** The name a payment may be thanked by in public, or null. */
    private String publicName(Payment payment) throws SQLException {
        if (payment.revoked) {
            return null;
        }
        if (payment.account != null) {
            return store.onWall(payment.account) ? payment.account : null;
        }
        return payment.isPublic && payment.fromName != null && !payment.fromName.trim().isEmpty() ? payment.fromName.trim() : null;
    }

    /** The lines of the costs setting: a JSON list, or text split at " · " and new lines. */
    static List<String> costs(String text) {
        List<String> lines = new ArrayList<>();
        if (text == null) {
            return lines;
        }
        String trimmed = text.trim();
        if (trimmed.startsWith("[")) {
            try {
                JsonArray array = JsonParser.parseString(trimmed).getAsJsonArray();
                for (JsonElement line : array) {
                    if (line.isJsonPrimitive() && !line.getAsString().trim().isEmpty()) {
                        lines.add(line.getAsString().trim());
                    }
                }
                return lines;
            } catch (JsonParseException | IllegalStateException e) {
                // plain text after all
            }
        }
        for (String line : trimmed.split("\\s*(·|\\r?\\n)\\s*")) {
            if (!line.isEmpty()) {
                lines.add(line);
            }
        }
        return lines;
    }

    static YearMonth month(long now) {
        return YearMonth.from(Instant.ofEpochMilli(now).atZone(ZoneOffset.UTC));
    }

    static long start(YearMonth month) {
        return month.atDay(1).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli();
    }

    // ---- staff

    /** A month of the ledger (2026-10; empty: this month), or every unmatched payment; narrowed to a provider. */
    public synchronized SupportLedger ledger(String monthText, boolean unmatchedOnly, String provider, long now) throws SQLException {
        ensurePeriods(now);
        YearMonth month;
        try {
            month = monthText == null || monthText.trim().isEmpty() ? month(now) : YearMonth.parse(monthText.trim());
        } catch (DateTimeParseException e) {
            throw new IllegalArgumentException("The month is written like 2026-10");
        }
        List<Payment> monthPayments = store.paymentsBetween(start(month), start(month.plusMonths(1)));
        SupportLedger ledger = new SupportLedger();
        ledger.month = month.toString();
        ledger.goalCents = RuntimeSettings.get().supportGoalCents();
        ledger.monthCents = usdCents(monthPayments);
        ledger.supporters = supporters(monthPayments);
        Map<String, Long> other = new TreeMap<>();
        for (Payment payment : monthPayments) {
            if (!payment.revoked && !"USD".equals(payment.currency)) {
                other.merge(payment.currency, payment.amountCents, Long::sum);
            }
        }
        ledger.otherCurrencies = new ArrayList<>();
        other.forEach((currency, cents) -> ledger.otherCurrencies.add(currency + " " + amount(cents)));
        ledger.unmatched = store.unmatchedCount();
        List<Payment> shown = unmatchedOnly ? store.unmatched(500) : monthPayments;
        ledger.payments = new ArrayList<>();
        for (Payment payment : shown) {
            if (provider == null || provider.trim().isEmpty() || provider.trim().equalsIgnoreCase(payment.provider)) {
                ledger.payments.add(payment);
            }
        }
        return ledger;
    }

    /** Staff link an unmatched payment to an account: it unlocks the perks like an automatic link. */
    public synchronized Payment link(long paymentId, String userName, long now) throws SQLException {
        Payment payment = store.payment(paymentId);
        if (payment == null) {
            throw new IllegalArgumentException("There is no payment " + paymentId);
        }
        if (payment.revoked) {
            throw new IllegalArgumentException("Payment " + paymentId + " was revoked");
        }
        String account = userName == null ? null : accounts.name(userName.trim());
        if (account == null) {
            throw new IllegalArgumentException("There is no account named " + userName);
        }
        store.linkPayment(paymentId, account, "staff");
        granted(account, paymentId, now, true);
        changed();
        return store.payment(paymentId);
    }

    /**
     * Staff revoke a payment (a refund or a chargeback): it stops counting. With {@code removePerks} the perks it
     * granted go too, unless another payment of the account still stands for them.
     */
    public synchronized Payment revoke(long paymentId, String reason, boolean removePerks, long now) throws SQLException {
        Payment payment = store.payment(paymentId);
        if (payment == null) {
            throw new IllegalArgumentException("There is no payment " + paymentId);
        }
        store.revokePayment(paymentId, reason);
        if (removePerks && payment.account != null && store.removePerksOf(paymentId) > 0) {
            for (Payment other : store.paymentsOf(payment.account)) {
                if (!other.revoked && other.id != paymentId) {
                    granted(payment.account, other.id, now, false);
                    break;
                }
            }
        }
        changed();
        return store.payment(paymentId);
    }

    /** Staff grant the thank-you set by hand (a payment made some other way). */
    public synchronized void grant(String userName, long now) throws SQLException {
        String account = userName == null ? null : accounts.name(userName.trim());
        if (account == null) {
            throw new IllegalArgumentException("There is no account named " + userName);
        }
        granted(account, 0, now, true);
    }

    // ---- the account's data

    /** Everything support keeps about the account, for its export: payments without the providers' payloads. */
    public synchronized Map<String, Object> export(String account) throws SQLException {
        Map<String, Object> out = new LinkedHashMap<>();
        out.put("code", store.codeOf(account));
        out.put("payments", store.paymentsOf(account));
        out.put("sponsorships", store.sponsorshipsOf(account));
        List<Map<String, Object>> perks = new ArrayList<>();
        for (SupportStore.OwnedPerk owned : store.perks(account)) {
            Map<String, Object> perk = new LinkedHashMap<>();
            perk.put("kind", owned.kind);
            perk.put("item", owned.item);
            perk.put("grantedAt", owned.grantedAt);
            perks.add(perk);
        }
        out.put("perks", perks);
        out.put("look", store.look(account));
        out.put("supportersWall", store.onWall(account));
        return out;
    }

    /** The account is deleted: its payments stay in the totals, anonymized; its perks and look go. */
    public synchronized void forget(String account) throws SQLException {
        store.forget(account);
        changed();
    }

    // ---- housekeeping

    /** Drops payloads older than 90 days, adds running sponsorships' months, ends finished ones. */
    public void daily(long now) throws SQLException {
        int dropped = store.dropPayloadsBefore(now - PAYLOAD_MILLIS);
        if (dropped > 0) {
            logger.info("Support: dropped " + dropped + " payload(s) older than 90 days");
        }
        ensurePeriods(now);
        changed();
    }

    /** Runs {@link #daily} a minute after startup and every six hours after. */
    public static void startDaily() {
        ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "XMAGE support daily");
            thread.setDaemon(true);
            return thread;
        });
        timer.scheduleAtFixedRate(() -> {
            try {
                get().daily(System.currentTimeMillis());
            } catch (SQLException | RuntimeException e) {
                logger.error("Support housekeeping failed", e);
            }
        }, 1, 6 * 60, TimeUnit.MINUTES);
    }

    // ---- telling people

    private static void logRecorded(Payment payment) {
        // amounts and kinds only: no names, messages or addresses in the log
        logger.info("Support: " + payment.provider + " " + payment.kind + " " + amount(payment.amountCents) + " " + payment.currency
                + " #" + payment.id + (payment.account == null ? " unmatched" : " linked"));
    }

    private void announce(Payment payment) throws SQLException {
        String name = publicName(payment);
        if (name == null || thanks == null) {
            return;
        }
        boolean amounts = RuntimeSettings.get().supportDiscordAmounts();
        String amount = !amounts ? null : "USD".equals(payment.currency) ? "$" + amount(payment.amountCents)
                : payment.currency + " " + amount(payment.amountCents);
        try {
            thanks.supported(name, amount);
        } catch (RuntimeException e) {
            logger.warn("Support thank-you post failed: " + e.getMessage());
        }
    }

    static String amount(long cents) {
        return String.format(Locale.ROOT, "%d.%02d", cents / 100, cents % 100);
    }

    private static String clip(String text, int max) {
        if (text == null) {
            return null;
        }
        String trimmed = text.trim();
        if (trimmed.isEmpty()) {
            return null;
        }
        return trimmed.length() <= max ? trimmed : trimmed.substring(0, max);
    }

    // ---- the server's accounts

    static Accounts serverAccounts() {
        return new Accounts() {
            @Override
            public String name(String userName) {
                AuthorizedUser account = userName == null ? null : AuthorizedUserRepository.getInstance().getByName(userName.trim());
                return account == null ? null : account.getName();
            }

            @Override
            public String confirmedEmailOwner(String email) {
                AuthorizedUserRepository repository = AuthorizedUserRepository.getInstance();
                AuthorizedUser account = repository.getByEmail(email);
                if (account == null) {
                    account = repository.getByEmail(email.toLowerCase(Locale.ROOT));
                }
                return account != null && EmailVerification.isVerified(account.getName(), account.getEmail()) ? account.getName() : null;
            }

            @Override
            public String githubAccount(long githubId) {
                try {
                    GithubLink link = AccessPolicy.get().store().githubLink(githubId);
                    return link == null ? null : name(link.account);
                } catch (SQLException | IllegalStateException e) {
                    return null;
                }
            }

            @Override
            public List<String[]> careerUnlocks(String account) {
                try {
                    return CareerStore.get().unlocksOf(account);
                } catch (SQLException | IllegalStateException e) {
                    return Collections.emptyList();
                }
            }

            @Override
            public Set<String> careerBadges(String account) {
                try {
                    return mage.server.career.CareerProgress.get().badges(account);
                } catch (SQLException | RuntimeException e) {
                    return Collections.emptySet();
                }
            }

            @Override
            public java.util.Map<String, String> careerCardStyles(String account) {
                try {
                    return mage.server.career.CareerCapsules.get().worn(account);
                } catch (SQLException | RuntimeException e) {
                    return Collections.emptyMap();
                }
            }
        };
    }
}

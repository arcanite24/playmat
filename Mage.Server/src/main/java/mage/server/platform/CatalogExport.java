package mage.server.platform;

import mage.cards.repository.CardCriteria;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.cards.repository.ExpansionInfo;
import mage.cards.repository.ExpansionRepository;
import mage.server.websocket.rpc.JsonCodec;
import mage.server.websocket.service.CardSearchService;
import mage.view.CardView;
import org.apache.log4j.Logger;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.zip.Deflater;

/**
 * Writes the card catalog the Rust platform service answers card lookups from (docs/RustMigration.md phase 2,
 * docs/rust/Catalog.md): every printing's view as the bridge sends it, the name and set-and-number indexes as the
 * card repository resolves them, and the set lists.
 * <p>
 * {@code -Dxmage.catalog.export=<file>}: written after startup when the file is missing or was made from other card
 * data, to a temporary file first and then moved in place, so a reader sees the old file or the new one.
 */
public final class CatalogExport {

    private static final Logger logger = Logger.getLogger(CatalogExport.class);

    public static final String PROP = "xmage.catalog.export";
    public static final int VERSION = 2;

    private CatalogExport() {
    }

    /** the stamp a catalog made from the current card data carries */
    static String stamp() {
        return VERSION + ":" + CardRepository.instance.getContentVersionFromDB() + ":"
                + CardRepository.instance.getContentVersionConstant() + ":" + ExpansionRepository.instance.getContentVersionFromDB();
    }

    /** Exports when {@link #PROP} is set and the file is out of date; the background thread unless {@code now}. */
    public static void startIfAsked(boolean now) {
        String path = System.getProperty(PROP);
        if (path == null || path.trim().isEmpty()) {
            return;
        }
        Runnable export = () -> {
            try {
                exportIfStale(new File(path.trim()));
            } catch (Exception e) {
                logger.error("Catalog export to " + path + " failed", e);
            }
        };
        if (now) {
            export.run();
        } else {
            Thread thread = new Thread(export, "Catalog export");
            thread.setDaemon(true);
            thread.setPriority(Thread.MIN_PRIORITY);
            thread.start();
        }
    }

    static boolean exportIfStale(File file) throws SQLException, java.io.IOException {
        String stamp = stamp();
        if (file.exists() && stamp.equals(readStamp(file))) {
            logger.info("Catalog " + file + " is current");
            return false;
        }
        long started = System.currentTimeMillis();
        File parent = file.getAbsoluteFile().getParentFile();
        if (parent != null && !parent.exists() && !parent.mkdirs()) {
            throw new java.io.IOException("Can't create " + parent);
        }
        File temp = new File(file.getPath() + ".tmp");
        Files.deleteIfExists(temp.toPath());
        int printings = write(temp, stamp);
        Files.move(temp.toPath(), file.toPath(), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        logger.info("Catalog " + file + ": " + printings + " printings in " + (System.currentTimeMillis() - started) + " ms");
        return true;
    }

    static String readStamp(File file) {
        try (Connection db = DriverManager.getConnection("jdbc:sqlite:" + file.getPath());
             Statement query = db.createStatement();
             ResultSet row = query.executeQuery("SELECT value FROM meta WHERE key = 'stamp'")) {
            return row.next() ? row.getString(1) : null;
        } catch (SQLException e) {
            return null;
        }
    }

    /** the database ignores case: the catalog's keys are lower case, and Rust lowers what it looks up */
    static String lower(String text) {
        return text.toLowerCase(Locale.ROOT);
    }

    private static String key(CardInfo info) {
        return info.getSetCode() + "|" + info.getCardNumber() + "|" + info.getName();
    }

    static int write(File file, String stamp) throws SQLException {
        try (Connection db = DriverManager.getConnection("jdbc:sqlite:" + file.getPath())) {
            try (Statement ddl = db.createStatement()) {
                ddl.execute("PRAGMA journal_mode = OFF");
                ddl.execute("PRAGMA synchronous = OFF");
                ddl.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value BLOB NOT NULL)");
                ddl.execute("CREATE TABLE printings (id INTEGER PRIMARY KEY, set_code TEXT NOT NULL, card_number TEXT NOT NULL,"
                        + " name TEXT NOT NULL, view BLOB NOT NULL)");
                ddl.execute("CREATE TABLE exact (set_code TEXT NOT NULL, card_number TEXT NOT NULL, printing INTEGER NOT NULL,"
                        + " PRIMARY KEY (set_code, card_number))");
                ddl.execute("CREATE TABLE names (query TEXT NOT NULL, kind INTEGER NOT NULL, position INTEGER NOT NULL,"
                        + " printing INTEGER NOT NULL, PRIMARY KEY (query, kind, position))");
                ddl.execute("CREATE TABLE sets (code TEXT PRIMARY KEY, standard_legal INTEGER NOT NULL, release_date INTEGER NOT NULL)");
                ddl.execute("CREATE TABLE results (method TEXT PRIMARY KEY, json TEXT NOT NULL)");
            }
            db.setAutoCommit(false);
            List<CardInfo> all = CardRepository.instance.findCards(new CardCriteria());
            byte[] dictionary = dictionary(all);
            // by lower case: the database finds the same printings whatever the case
            Map<String, String> queries = new java.util.LinkedHashMap<>();
            Printings printings;
            try (PreparedStatement insert = db.prepareStatement("INSERT INTO printings VALUES (?, ?, ?, ?, ?)")) {
                printings = new Printings(insert, dictionary);
                for (CardInfo info : all) {
                    printings.id(info);
                    queries.putIfAbsent(lower(info.getName()), info.getName());
                    for (String other : new String[]{info.getFlipCardName(), info.getSecondSideName(),
                            info.getSpellOptionCardName(), info.getDoubleFacedSecondSideName()}) {
                        if (other != null && !other.isEmpty()) {
                            queries.putIfAbsent(lower(other), other);
                        }
                    }
                    if (info.getName().contains(" // ")) {
                        for (String half : info.getName().split(" // ")) {
                            queries.putIfAbsent(lower(half), half);
                        }
                    }
                }
                insert.executeBatch();

                // set and number as findCard resolves them (the main side first)
                try (PreparedStatement exact = db.prepareStatement("INSERT INTO exact VALUES (?, ?, ?)")) {
                    for (String setAndNumber : CardRepository.instance.getSetAndNumberKeys()) {
                        // lower case, as the database compares them
                        String[] parts = setAndNumber.split("\t", 2);
                        CardInfo info = CardRepository.instance.findCard(parts[0], parts[1]);
                        if (info == null) {
                            continue;
                        }
                        exact.setString(1, parts[0]);
                        exact.setString(2, parts[1]);
                        exact.setInt(3, printings.id(info));
                        exact.addBatch();
                    }
                    insert.executeBatch();
                    exact.executeBatch();
                }

                // kind 0: findCards(query), what a name lookup gets; kind 1: printings named exactly that, what the
                // first half of an unknown "A // B" falls back to (any name a card or one of its halves has); kind 2:
                // the printing findPreferredCoreExpansionCard(query) picks, what a deck that only names a card gets
                try (PreparedStatement names = db.prepareStatement("INSERT INTO names VALUES (?, ?, ?, ?)")) {
                    for (String query : queries.values()) {
                        names(names, printings, query, 0, CardRepository.instance.findCards(query));
                        CardInfo preferred = CardRepository.instance.findPreferredCoreExpansionCard(query, "");
                        if (preferred != null) {
                            names(names, printings, query, 2, java.util.Collections.singletonList(preferred));
                        }
                    }
                    for (String name : queries.values()) {
                        names(names, printings, name, 1, CardRepository.instance.findCardsNamed(name));
                    }
                    insert.executeBatch();
                    names.executeBatch();
                }
            }
            try (PreparedStatement sets = db.prepareStatement("INSERT INTO sets VALUES (?, ?, ?)")) {
                for (ExpansionInfo set : ExpansionRepository.instance.getAll()) {
                    sets.setString(1, set.getCode());
                    sets.setInt(2, set.getType().isStandardLegal() ? 1 : 0);
                    sets.setLong(3, set.getReleaseDate() == null ? 0L : set.getReleaseDate().getTime());
                    sets.addBatch();
                }
                sets.executeBatch();
            }
            CardSearchService search = new CardSearchService();
            try (PreparedStatement results = db.prepareStatement("INSERT INTO results VALUES (?, ?)")) {
                results.setString(1, "getExpansionSets");
                results.setString(2, JsonCodec.GSON.toJson(search.expansionSets()));
                results.addBatch();
                results.setString(1, "getBasicLandSets");
                results.setString(2, JsonCodec.GSON.toJson(search.basicLandSets()));
                results.addBatch();
                results.executeBatch();
            }
            try (PreparedStatement meta = db.prepareStatement("INSERT INTO meta VALUES (?, ?)")) {
                meta.setString(1, "stamp");
                meta.setString(2, stamp);
                meta.addBatch();
                meta.setString(1, "exportedAt");
                meta.setString(2, String.valueOf(System.currentTimeMillis()));
                meta.addBatch();
                // views are raw deflate with this preset dictionary
                meta.setString(1, "viewDictionary");
                meta.setBytes(2, dictionary);
                meta.addBatch();
                meta.executeBatch();
            }
            db.commit();
            return printings.count();
        }
    }

    /**
     * Views of a few printings spread over the database, 32 kB at most (deflate's window): every view shares most
     * of its text with them, so each compresses to about a ninth on its own.
     */
    static byte[] dictionary(List<CardInfo> all) {
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        int step = Math.max(1, all.size() / 24);
        for (int i = 0; i < all.size(); i += step) {
            byte[] view = view(all.get(i));
            out.write(view, 0, view.length);
        }
        byte[] bytes = out.toByteArray();
        return Arrays.copyOfRange(bytes, Math.max(0, bytes.length - 32768), bytes.length);
    }

    static byte[] view(CardInfo info) {
        return JsonCodec.GSON.toJson(new CardView(info.createMockCard())).getBytes(StandardCharsets.UTF_8);
    }

    /** the printings table: one row per printing (set, number and name), numbered from 1 */
    private static final class Printings {

        private final PreparedStatement insert;
        private final byte[] dictionary;
        private final Map<String, Integer> ids = new HashMap<>();
        private final Deflater deflater = new Deflater(Deflater.BEST_COMPRESSION, true);
        private final byte[] buffer = new byte[256 * 1024];

        Printings(PreparedStatement insert, byte[] dictionary) {
            this.insert = insert;
            this.dictionary = dictionary;
        }

        int count() {
            return ids.size();
        }

        int id(CardInfo info) throws SQLException {
            String key = key(info);
            Integer id = ids.get(key);
            if (id != null) {
                return id;
            }
            id = ids.size() + 1;
            ids.put(key, id);
            insert.setInt(1, id);
            insert.setString(2, info.getSetCode());
            insert.setString(3, info.getCardNumber());
            insert.setString(4, info.getName());
            insert.setBytes(5, compress(view(info)));
            insert.addBatch();
            if (ids.size() % 2000 == 0) {
                insert.executeBatch();
            }
            return id;
        }

        private byte[] compress(byte[] view) {
            deflater.reset();
            deflater.setDictionary(dictionary);
            deflater.setInput(view);
            deflater.finish();
            int length = deflater.deflate(buffer);
            if (!deflater.finished()) {
                throw new IllegalStateException("card view too large: " + view.length + " bytes");
            }
            return Arrays.copyOf(buffer, length);
        }
    }

    private static void names(PreparedStatement names, Printings printings, String query, int kind,
                              List<CardInfo> found) throws SQLException {
        int position = 0;
        for (CardInfo info : found) {
            names.setString(1, lower(query));
            names.setInt(2, kind);
            names.setInt(3, position++);
            names.setInt(4, printings.id(info));
            names.addBatch();
        }
        if (position > 0) {
            names.executeBatch();
        }
    }
}

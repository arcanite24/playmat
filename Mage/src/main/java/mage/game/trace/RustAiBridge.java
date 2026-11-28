package mage.game.trace;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.constants.ManaType;
import mage.game.Game;
import org.apache.log4j.Logger;

import java.io.*;
import java.lang.management.ManagementFactory;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;

/**
 * The Rust AI playing one seat of a game Java hosts (docs/rust/Ai.md): a {@code playmat-ai-bridge} process keeps a
 * Rust copy of the game in step and answers that seat's questions.
 * <p>
 * The game is traced and its answers logged ({@link mage.game.GameOptions#trace}, {@link mage.game.GameOptions#decisions}). At the first
 * trace entry the bridge starts the copy from {@link ShadowStart}; from then on it forwards the other seat's answers
 * and every trace entry, and the seat's dialogs ({@code ComputerPlayerControllableProxy}) ask it for their answers.
 * When the copy differs from Java, the process dies or an answer takes too long, the bridge stops for good and the
 * seat goes back to the Java AI.
 */
public final class RustAiBridge {

    private static final Logger logger = Logger.getLogger(RustAiBridge.class);
    private static final long ANSWER_TIMEOUT_MS = 20_000;
    private static final Map<String, Boolean> SUPPORTED = new ConcurrentHashMap<>();
    private static final Gson GSON = new Gson();
    /** PLAYMAT_AI_DEBUG: the bridge logs each question, and gets Java's fingerprint texts to show a difference */
    private static final boolean DEBUG = System.getenv("PLAYMAT_AI_DEBUG") != null;

    public static final AtomicLong STARTED = new AtomicLong();
    public static final AtomicLong ANSWERED = new AtomicLong();
    public static final AtomicLong STOPPED = new AtomicLong();
    public static final AtomicLong FINISHED = new AtomicLong();
    /** the other seat's views the copies compared, and those with a field that differs (docs/rust/Views.md) */
    public static final AtomicLong VIEWS_COMPARED = new AtomicLong();
    public static final AtomicLong VIEWS_DIFFERING = new AtomicLong();
    /**
     * The view a player's client gets now, ids mapped to decision-log references, as JSON (set by the server: the
     * views live there); null: no views are compared
     */
    public static volatile java.util.function.BiFunction<Game, UUID, Object> VIEWS;
    /** the human's prompts the copies rendered and compared with Java's, and those with a field that differs */
    public static final AtomicLong PROMPTS_COMPARED = new AtomicLong();
    public static final AtomicLong PROMPTS_DIFFERING = new AtomicLong();
    /** the game log's lines the copies compared with their own, and those the copy didn't write the same */
    public static final AtomicLong LOG_LINES_COMPARED = new AtomicLong();
    public static final AtomicLong LOG_LINES_DIFFERING = new AtomicLong();
    /**
     * The prompt a player's client gets with a question ({@code {method, data}}, ids mapped), as JSON, or null for
     * one that isn't rendered (set by the server); null: no prompts are compared
     */
    public static volatile java.util.function.BiFunction<Game, mage.game.events.PlayerQueryEvent, Object> PROMPTS;
    /** a game log line, ids mapped and players named by seat (set by the server); null: the log isn't compared */
    public static volatile java.util.function.BiFunction<Game, String, String> LOG;
    /** XMAGE_RUST_PROMPTS: 0 turns the prompts' and the log's comparison off */
    private static final boolean PROMPTS_ON = !"0".equals(String.valueOf(System.getenv("XMAGE_RUST_PROMPTS")).trim());
    /** XMAGE_RUST_VIEWS_EVERY: the copy compares the other seat's view at every n-th question to it (0: never) */
    private static final int VIEWS_EVERY = viewsEvery();

    private static int viewsEvery() {
        try {
            String v = System.getenv("XMAGE_RUST_VIEWS_EVERY");
            return v == null || v.trim().isEmpty() ? 10 : Math.max(0, Integer.parseInt(v.trim()));
        } catch (NumberFormatException e) {
            return 10;
        }
    }

    /** the other seat's questions so far, for the views' sampling */
    private int otherAsked;

    private final AtomicLong answered = new AtomicLong();
    /** the game thread's CPU when the copy started; the Java side's cost at the end; the copy's CPU when it reports */
    private long gameThread;
    private long gameThreadCpu;
    private long[] javaCost;
    private volatile long copyCpuMillis = -1;
    private boolean costLogged;

    private final UUID seatId;
    private final DecisionLog log;
    private final String gameName;
    private Process process;
    private Writer out;
    private final BlockingQueue<String> lines = new LinkedBlockingQueue<>();
    private int seat = -1;
    private int sent;
    private volatile boolean active;
    private volatile String failure;
    /** the last question Java put to the seat (for the bridge's messages) */
    private volatile String question = "";
    /** the choice it was, for a CHOOSE_CHOICE */
    private volatile mage.choices.Choice choice;

    /**
     * @param seatId the player the Rust AI plays
     * @param log    the game's decision log (recording)
     */
    public RustAiBridge(UUID seatId, DecisionLog log, String gameName) {
        this.seatId = seatId;
        this.log = log;
        this.gameName = gameName;
    }

    /** the process: {@code -Dxmage.rustAi.bin} or {@code XMAGE_RUST_AI_BIN}, else {@code playmat-ai-bridge} on the path */
    public static String binary() {
        String bin = System.getProperty("xmage.rustAi.bin", System.getenv("XMAGE_RUST_AI_BIN"));
        return bin == null || bin.isEmpty() ? "playmat-ai-bridge" : bin;
    }

    /**
     * Whether a game with these cards may go to Rust: the engine has every one and each is verified against Java
     * ({@code cards/verified.txt}; asked once per name, then remembered).
     */
    public static boolean supports(Collection<String> names) {
        return missing(names).isEmpty();
    }

    /**
     * The cards of these a game can't go to Rust with: the engine doesn't have them, or has them without their being
     * verified against Java ({@code cards/verified.txt}); asked once per name, then remembered. All of them when the
     * bridge can't be asked.
     */
    public static Set<String> missing(Collection<String> names) {
        List<String> unknown = new ArrayList<>();
        for (String n : new LinkedHashSet<>(names)) {
            if (!SUPPORTED.containsKey(n)) {
                unknown.add(n);
            }
        }
        if (!unknown.isEmpty()) {
            try {
                Process p = new ProcessBuilder(binary(), "--check").redirectErrorStream(false).start();
                try (Writer w = new OutputStreamWriter(p.getOutputStream(), StandardCharsets.UTF_8)) {
                    for (String n : unknown) {
                        w.write(n.replace('\n', ' ') + "\n");
                    }
                }
                Set<String> missing = new HashSet<>();
                try (BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream(), StandardCharsets.UTF_8))) {
                    String line;
                    while ((line = r.readLine()) != null) {
                        missing.add(line.trim());
                    }
                }
                if (!p.waitFor(30, TimeUnit.SECONDS) || p.exitValue() != 0) {
                    p.destroyForcibly();
                    return new LinkedHashSet<>(names);
                }
                for (String n : unknown) {
                    SUPPORTED.put(n, !missing.contains(n));
                }
            } catch (IOException e) {
                logger.warn("Rust AI: can't run " + binary() + ": " + e.getMessage());
                return new LinkedHashSet<>(names);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return new LinkedHashSet<>(names);
            }
        }
        Set<String> missing = new TreeSet<>();
        for (String n : names) {
            if (!SUPPORTED.getOrDefault(n, false)) {
                missing.add(n);
            }
        }
        return missing;
    }

    /** the trace's sink: starts the copy at the first entry, forwards the rest, ends with the game */
    public void onTrace(Game game, GameTrace.Entry entry) {
        try {
            if (entry.index == 0) {
                start(game);
            }
            if (active) {
                sendDecisions();
                JsonObject m = new JsonObject();
                m.addProperty("type", "trace");
                m.addProperty("fp", entry.hash);
                if (entry.text != null) {
                    remember(entry.hash, entry.text);
                }
                if (DEBUG && entry.text != null) {
                    m.addProperty("text", entry.text);
                }
                send(m.toString());
            }
        } catch (RuntimeException e) {
            stop("trace: " + e);
        }
    }

    /** the last states' texts by hash, to show where the copy's state differs */
    private final Map<String, String> texts = new LinkedHashMap<String, String>() {
        @Override
        protected boolean removeEldestEntry(Map.Entry<String, String> eldest) {
            return size() > 32;
        }
    };

    private synchronized void remember(String hash, String text) {
        texts.put(hash, text);
    }

    /** a "diverged" at a differing state: the lines only Java's state has, and those only the copy's has */
    private synchronized void logDifference(JsonObject a) {
        if (!a.has("javaFp") || !a.has("rustText")) {
            return;
        }
        String java = texts.get(a.get("javaFp").getAsString());
        if (java == null) {
            return;
        }
        List<String> j = new ArrayList<>(Arrays.asList(java.split("\n")));
        List<String> r = new ArrayList<>(Arrays.asList(a.get("rustText").getAsString().split("\n")));
        List<String> onlyJava = new ArrayList<>(j);
        for (String line : r) {
            onlyJava.remove(line);
        }
        List<String> onlyRust = new ArrayList<>(r);
        for (String line : j) {
            onlyRust.remove(line);
        }
        logger.info("Rust AI " + gameName + ": " + j.get(Math.min(1, j.size() - 1)) + "\n  Java only:\n    "
                + String.join("\n    ", onlyJava) + "\n  Rust only:\n    " + String.join("\n    ", onlyRust));
    }

    /** the answers this bridge gave */
    public long getAnswered() {
        return answered.get();
    }

    public boolean isActive() {
        return active;
    }

    public String getFailure() {
        return failure;
    }

    private void start(Game game) {
        gameThread = Thread.currentThread().getId();
        gameThreadCpu = GameTrace.cpuNanos();
        seat = StateFingerprint.seats(game).indexOf(seatId);
        if (seat < 0) {
            stop("the seat isn't in the game");
            return;
        }
        for (UUID id : StateFingerprint.seats(game)) {
            if (game.getPlayer(id).getLife() != 20) {
                stop("starting life isn't 20");
                return;
            }
        }
        Map<String, Object> start = ShadowStart.capture(game, log, StateFingerprint.text(game));
        sent = (Integer) start.get("decision");
        Map<String, Object> message = new LinkedHashMap<>();
        message.put("type", "start");
        message.put("seat", seat);
        message.put("start", start);
        // Java's AI players auto-choose targets at level 2 (TargetImpl.tryToAutoChoose)
        message.put("autoTargetAll", true);
        // the other seat's skip settings and pass-until actions aren't the copy's: it logs every pass, and the copy
        // asks it at every priority
        log.recordAutoPasses();
        message.put("everyPriority", true);
        // the copy keeps its game log to compare with the lines it gets
        message.put("log", PROMPTS_ON && LOG != null);
        for (UUID id : StateFingerprint.seats(game)) {
            if (!id.equals(seatId) && game.getPlayer(id).getUserData() != null) {
                message.put("poolCheck", game.getPlayer(id).getUserData().confirmEmptyManaPool());
            }
        }
        try {
            process = new ProcessBuilder(binary()).start();
        } catch (IOException e) {
            stop("can't run " + binary() + ": " + e.getMessage());
            return;
        }
        out = new BufferedWriter(new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
        Thread reader = new Thread(() -> {
            try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = r.readLine()) != null) {
                    lines.add(line);
                }
            } catch (IOException ignore) {
            }
            lines.add("{\"type\":\"closed\"}");
        }, "rust-ai " + gameName);
        reader.setDaemon(true);
        reader.start();
        Thread errors = new Thread(() -> {
            try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getErrorStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = r.readLine()) != null) {
                    if (line.startsWith("views ")) {
                        logViews(line.substring(6));
                    } else if (line.startsWith("prompts ")) {
                        logTally("prompts", line.substring(8), PROMPTS_COMPARED, PROMPTS_DIFFERING);
                    } else if (line.startsWith("log ")) {
                        logTally("log lines", line.substring(4), LOG_LINES_COMPARED, LOG_LINES_DIFFERING);
                    } else if (line.startsWith("cpu ") && line.endsWith(" ms")) {
                        copyCpuMillis = Long.parseLong(line.substring(4, line.length() - 3));
                        logCost();
                    } else {
                        logger.info("Rust AI " + gameName + ": " + line);
                    }
                }
            } catch (IOException ignore) {
            }
        }, "rust-ai-err " + gameName);
        errors.setDaemon(true);
        errors.start();
        game.addPlayerQueryEventListener(event -> {
            if (event.getPlayerId() != null && !seatId.equals(event.getPlayerId())) {
                sendView(game, event.getPlayerId());
                sendPrompt(game, event);
            }
            if (seatId.equals(event.getPlayerId())) {
                choice = event.getChoice();
                question = event.getQueryType() + " " + (event.getMessage() == null ? "" : event.getMessage())
                        // a choice's options, always: the copy answers a mana color from them
                        + (event.getChoice() != null ? " " + event.getChoice().getKeyChoices() + event.getChoice().getChoices() : "");
            }
        });
        active = true;
        send(GSON.toJson(message));
        JsonObject ready = next();
        if (ready == null || !"ready".equals(type(ready))) {
            stop("didn't start: " + (ready == null ? failure : ready));
            return;
        }
        STARTED.incrementAndGet();
        logger.info("Rust AI plays seat " + seat + " of " + gameName);
    }

    /**
     * The Rust AI's answer to the question its seat is being asked; null when the bridge has stopped (the dialog
     * then ends, and the seat goes back to the Java AI).
     */
    public synchronized DecisionLog.Answer ask(Game game) {
        if (!active) {
            return null;
        }
        sendDecisions();
        JsonObject ask = new JsonObject();
        ask.addProperty("type", "ask");
        ask.addProperty("question", question);
        send(ask.toString());
        JsonObject a = next();
        if (a == null) {
            return null;
        }
        if (!"answer".equals(type(a))) {
            logDifference(a);
            stop(type(a) + ": " + (a.has("message") ? a.get("message").getAsString() : a.toString()));
            return null;
        }
        DecisionLog.Answer answer = new DecisionLog.Answer();
        if (a.has("target") && !a.get("target").isJsonNull()) {
            String ref = a.get("target").getAsString();
            answer.target = DecisionLog.resolve(game, ref);
            if (answer.target == null) {
                stop("the answer " + ref + " names nothing in Java's game");
                return null;
            }
            // a Room's half or a card's face names the card; the cast dialog wants its spell
            mage.cards.Card card = question.startsWith("CHOOSE_ABILITY") ? game.getCard(answer.target) : null;
            if (card != null && card.getSpellAbility() != null) {
                answer.target = card.getSpellAbility().getId();
            }
        }
        if (a.has("bool") && !a.get("bool").isJsonNull()) {
            answer.bool = a.get("bool").getAsBoolean();
        }
        if (a.has("integer") && !a.get("integer").isJsonNull()) {
            answer.integer = a.get("integer").getAsInt();
        }
        if (a.has("string") && !a.get("string").isJsonNull()) {
            answer.string = a.get("string").getAsString();
        }
        if (a.has("manaType") && !a.get("manaType").isJsonNull()) {
            answer.manaType = ManaType.valueOf(a.get("manaType").getAsString());
        }
        // a choice the Rust copy doesn't list (a replacement effect's order, keyed by Java's ids): the first one
        mage.choices.Choice c = choice;
        if (answer.string != null && c != null && question.startsWith("CHOOSE_CHOICE")) {
            if (c.isKeyChoice() && !c.getKeyChoices().containsKey(answer.string) && !c.getKeyChoices().isEmpty()) {
                answer.string = c.getKeyChoices().keySet().iterator().next();
            } else if (!c.isKeyChoice() && !c.getChoices().contains(answer.string) && !c.getChoices().isEmpty()) {
                answer.string = c.getChoices().iterator().next();
            }
        }
        ANSWERED.incrementAndGet();
        answered.incrementAndGet();
        return answer;
    }

    /**
     * At the other seat's question: its view, for the copy to render its own at the same point and count the fields
     * that differ (it never stops the copy). Every {@link #VIEWS_EVERY}-th question, as building a view costs.
     */
    private void sendView(Game game, UUID playerId) {
        java.util.function.BiFunction<Game, UUID, Object> views = VIEWS;
        if (!active || views == null || VIEWS_EVERY <= 0 || otherAsked++ % VIEWS_EVERY != 0) {
            return;
        }
        try {
            Object view = views.apply(game, playerId);
            sendDecisions();
            JsonObject m = new JsonObject();
            m.addProperty("type", "view");
            m.add("view", view instanceof com.google.gson.JsonElement ? (com.google.gson.JsonElement) view
                    : JsonParser.parseString(String.valueOf(view)));
            com.google.gson.JsonArray names = new com.google.gson.JsonArray();
            com.google.gson.JsonArray humans = new com.google.gson.JsonArray();
            for (UUID id : StateFingerprint.seats(game)) {
                names.add(game.getPlayer(id).getName());
                humans.add(game.getPlayer(id).isHuman());
            }
            m.add("names", names);
            m.add("humans", humans);
            send(m.toString());
        } catch (RuntimeException e) {
            // a view is an extra: the copy goes on without it
            logger.warn("Rust AI " + gameName + ": no view to compare: " + e);
        }
    }

    /** At the other seat's question: the prompt its client gets, for the copy to render its own and compare */
    private void sendPrompt(Game game, mage.game.events.PlayerQueryEvent event) {
        java.util.function.BiFunction<Game, mage.game.events.PlayerQueryEvent, Object> prompts = PROMPTS;
        if (!active || !PROMPTS_ON || prompts == null) {
            return;
        }
        try {
            Object prompt = prompts.apply(game, event);
            if (prompt == null) {
                return;
            }
            sendDecisions();
            JsonObject m = new JsonObject();
            m.addProperty("type", "prompt");
            m.add("prompt", prompt instanceof com.google.gson.JsonElement ? (com.google.gson.JsonElement) prompt
                    : JsonParser.parseString(String.valueOf(prompt)));
            send(m.toString());
        } catch (RuntimeException e) {
            logger.warn("Rust AI " + gameName + ": no prompt to compare: " + e);
        }
    }

    /** A line of the game log ({@code game.informPlayers}), for the copy to compare with the lines it writes */
    public synchronized void onLog(Game game, String line) {
        java.util.function.BiFunction<Game, String, String> map = LOG;
        if (!active || !PROMPTS_ON || map == null || line == null) {
            return;
        }
        try {
            // after the answers it follows, as the copy counts them
            sendDecisions();
            JsonObject m = new JsonObject();
            m.addProperty("type", "log");
            m.addProperty("line", map.apply(game, line));
            send(m.toString());
        } catch (RuntimeException e) {
            logger.warn("Rust AI " + gameName + ": no log line to compare: " + e);
        }
    }

    /** the other seat's answers since the last call */
    private void sendDecisions() {
        List<DecisionLog.Decision> decisions = log.getDecisions();
        for (; sent < decisions.size(); sent++) {
            DecisionLog.Decision d = decisions.get(sent);
            if (d.seat == seat) {
                continue;
            }
            JsonObject m = new JsonObject();
            m.addProperty("type", "decision");
            m.addProperty("seat", d.seat);
            if (d.target != null) m.addProperty("target", d.target);
            if (d.bool != null) m.addProperty("bool", d.bool);
            if (d.integer != null) m.addProperty("integer", d.integer);
            if (d.string != null) m.addProperty("string", d.string);
            if (d.manaType != null) m.addProperty("manaType", d.manaType);
            send(m.toString());
        }
    }

    private void send(String line) {
        if (!active) {
            return;
        }
        try {
            out.write(line);
            out.write('\n');
            out.flush();
        } catch (IOException e) {
            stop("the process went away: " + e.getMessage());
        }
    }

    private JsonObject next() {
        try {
            String line = lines.poll(ANSWER_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            if (line == null) {
                stop("no answer in " + ANSWER_TIMEOUT_MS / 1000 + " s");
                return null;
            }
            JsonObject o = JsonParser.parseString(line).getAsJsonObject();
            if ("closed".equals(type(o))) {
                stop("the process ended");
                return null;
            }
            return o;
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            stop("interrupted");
            return null;
        } catch (RuntimeException e) {
            stop("bad answer: " + e.getMessage());
            return null;
        }
    }

    private static String type(JsonObject o) {
        return o.has("type") ? o.get("type").getAsString() : "";
    }

    /** stops for good: the seat plays on with the Java AI */
    public void stop(String why) {
        boolean was = active || process == null;
        active = false;
        if (failure == null) {
            failure = why;
            if (was) {
                STOPPED.incrementAndGet();
                logger.warn("Rust AI stops in " + gameName + " (the Java AI plays on): " + why);
            }
        }
        close();
    }

    /**
     * the game ended: the process is told and let go, and the game's cost is logged once the process reports its CPU:
     * the game thread's CPU since the copy started (Java's engine, its dialogs and views), the share of it the trace
     * took, and the copy's
     */
    public void end(Game game, GameTrace trace) {
        if (gameThreadCpu > 0) {
            synchronized (this) {
                // the end can come on another thread (a concede, a timeout)
                long now = ManagementFactory.getThreadMXBean().getThreadCpuTime(gameThread);
                javaCost = new long[]{Math.max(0, now - gameThreadCpu), trace.getFingerprintNanos(),
                        trace.getSinkNanos(), game.getTurnNum()};
            }
            logCost();
        }
        if (active && PROMPTS_ON && LOG != null) {
            // the game's last log lines come after its end (who lost and why, what died with them): the process
            // is told at the match's end-of-game info ({@link #end()}), or in a few seconds if that never comes
            Thread t = new Thread(() -> {
                try {
                    Thread.sleep(10_000);
                } catch (InterruptedException ignore) {
                }
                end();
            }, "rust-ai-end " + gameName);
            t.setDaemon(true);
            t.start();
            return;
        }
        end();
    }

    /** the copy's count of the views it compared: {@code {"compared":n,"differing":m,"fields":{path:count}}} */
    private void logViews(String json) {
        try {
            JsonObject v = JsonParser.parseString(json).getAsJsonObject();
            long compared = v.get("compared").getAsLong();
            long differing = v.get("differing").getAsLong();
            if (compared == 0) {
                return;
            }
            VIEWS_COMPARED.addAndGet(compared);
            VIEWS_DIFFERING.addAndGet(differing);
            logger.info("Rust views in " + gameName + ": " + compared + " compared, " + differing + " differ"
                    + (differing > 0 && v.has("fields") ? "; fields: " + v.get("fields") : "")
                    + (differing > 0 && v.has("examples") ? "\n  " + String.join("\n  ", examples(v)) : ""));
        } catch (RuntimeException e) {
            logger.info("Rust AI " + gameName + ": views " + json);
        }
    }

    /** the copy's count of the prompts or log lines it compared: {@code {"compared":n,"differing":m,…}} */
    private void logTally(String what, String json, AtomicLong compared, AtomicLong differing) {
        try {
            JsonObject v = JsonParser.parseString(json).getAsJsonObject();
            long n = v.get("compared").getAsLong();
            long d = v.get("differing").getAsLong();
            if (n == 0) {
                return;
            }
            compared.addAndGet(n);
            differing.addAndGet(d);
            logger.info("Rust " + what + " in " + gameName + ": " + n + " compared, " + d + " differ"
                    + (d > 0 && v.has("fields") ? "; " + v.get("fields") : "")
                    + (d > 0 && v.has("examples") ? "\n  " + String.join("\n  ", examples(v)) : ""));
        } catch (RuntimeException e) {
            logger.info("Rust AI " + gameName + ": " + what + " " + json);
        }
    }

    private static List<String> examples(JsonObject v) {
        List<String> out = new ArrayList<>();
        for (com.google.gson.JsonElement e : v.getAsJsonArray("examples")) {
            out.add(e.getAsString());
        }
        return out;
    }

    private synchronized void logCost() {
        if (javaCost == null || copyCpuMillis < 0 || costLogged) {
            return;
        }
        costLogged = true;
        logger.info("Rust AI cost in " + gameName + ": " + javaCost[3] + " turns, " + answered.get()
                + " answers; CPU ms: game thread " + javaCost[0] / 1_000_000 + " (fingerprints " + javaCost[1] / 1_000_000
                + ", sending to the copy " + javaCost[2] / 1_000_000 + "), copy " + copyCpuMillis
                + (failure != null ? " (stopped partway)" : ""));
    }

    /** the game ended: the process is told and let go */
    public synchronized void end() {
        if (active) {
            send("{\"type\":\"end\"}");
            active = false;
            FINISHED.incrementAndGet();
        }
        close();
    }

    private void close() {
        if (process != null) {
            try {
                out.close();
            } catch (IOException ignore) {
            }
            Process p = process;
            Thread t = new Thread(() -> {
                try {
                    if (!p.waitFor(5, TimeUnit.SECONDS)) {
                        p.destroyForcibly();
                    }
                } catch (InterruptedException e) {
                    p.destroyForcibly();
                }
            });
            t.setDaemon(true);
            t.start();
        }
    }
}

package mage.server;

import mage.cards.ExpansionSet;
import mage.cards.RateCard;
import mage.cards.Sets;
import mage.cards.decks.DeckValidatorFactory;
import mage.cards.repository.CardScanner;
import mage.cards.repository.PluginClassloaderRegistery;
import mage.cards.repository.RepositoryUtil;
import mage.server.draft.CubeFactory;
import mage.server.game.GameFactory;
import mage.server.game.PlayerFactory;
import mage.server.managers.ConfigSettings;
import mage.server.managers.ManagerFactory;
import mage.server.record.UserStatsRepository;
import mage.server.tournament.TournamentFactory;
import mage.server.util.*;
import mage.server.util.config.GamePlugin;
import mage.server.util.config.Plugin;
import mage.util.DebugUtil;
import mage.util.JavaUtil;
import mage.utils.MageVersion;
import mage.utils.SystemUtil;
import org.apache.log4j.Logger;

import java.io.File;
import java.io.IOException;
import java.nio.charset.Charset;
import java.nio.file.Paths;
import java.util.*;

/**
 * @author BetaSteward_at_googlemail.com, JayDi85
 */
public final class Main {

    static {
        // must be first java code
        JavaUtil.applyDefaultServerSettings();
    }

    private static final Logger logger = Logger.getLogger(Main.class);
    private static final MageVersion version = new MageVersion(Main.class);

    // arg settings can be setup by run script or IDE's program arguments like -xxx=yyy
    // prop settings can be setup by -Dxxx=yyy in the launcher
    // priority: default setting -> prop setting -> arg setting
    private static final String testModeArg = "-testMode=";
    private static final String testModeProp = "xmage.testMode";
    private static final String detailsModeArg = "-detailsMode=";
    private static final String detailsModeProp = "xmage.detailsMode";
    private static final String adminPasswordArg = "-adminPassword=";
    private static final String adminPasswordProp = "xmage.adminPassword";
    private static final String configPathProp = "xmage.config.path";
    // image builds start the server once to build the card database and to record the classes it loads, then stop
    private static final String exitAfterStartupProp = "xmage.exitAfterStartup";
    // the desktop protocol (JBoss Remoting) is gone: a server with this set to true says so and runs for web clients
    private static final String remotingProp = "xmage.remoting";

    private static final File extensionFolder = new File("extensions");
    private static final String defaultConfigPath = Paths.get("config", "config.xml").toString();

    // Special test mode:
    // - fast game buttons;
    // - cheat commands;
    // - no deck validation;
    // - no draft's clicks protection timeout;
    // - no connection validation by pings (no disconnects on IDE's debugger usage);
    // - load any deck in sideboarding;
    // - simplified registration and login (no password check);
    // - debug main menu for GUI and rendering testing (must use -debug arg for client app);
    private static boolean testMode;
    private static boolean detailsMode;

    public static void main(String[] args) {
        logger.info("Starting MAGE SERVER version: " + version);
        logger.info("Java version: " + System.getProperty("java.version"));
        DebugUtil.printLogsInfo(logger);
        logger.info("Default charset: " + Charset.defaultCharset());
        if (!Charset.defaultCharset().toString().equals("UTF-8")) {
            logger.warn("WARNING, bad charset. Some texts with non-ascii symbols can be broken (decks, server messages). You must:");
            logger.warn("* Open launcher -> settings -> java -> server java options");
            logger.warn("* Insert at the the end: -Dfile.encoding=UTF-8 -Dsun.jnu.encoding=UTF-8");
        }
        String adminPassword = "";

        // enable test mode by default for developer build (if you run it from source code)
        testMode |= version.isDeveloperBuild();
        detailsMode = false;

        // settings by properties (-Dxxx=yyy from a launcher)
        if (System.getProperty(testModeProp) != null) {
            testMode = Boolean.parseBoolean(System.getProperty(testModeProp));
        }
        if (System.getProperty(adminPasswordProp) != null) {
            adminPassword = SystemUtil.sanitize(System.getProperty(adminPasswordProp));
            System.clearProperty(adminPasswordProp); // nothing else in the server needs to read it
        }
        if (System.getProperty(detailsModeProp) != null) {
            detailsMode = Boolean.parseBoolean(System.getProperty(detailsModeProp));
        }
        final String configPath;
        if (System.getProperty(configPathProp) != null) {
            configPath = System.getProperty(configPathProp);
        } else {
            configPath = defaultConfigPath;
        }

        // settings by program arguments (-xxx=yyy from a command line)
        for (String arg : args) {
            if (arg.startsWith(testModeArg)) {
                testMode = Boolean.parseBoolean(arg.replace(testModeArg, ""));
            } else if (arg.startsWith(adminPasswordArg)) {
                adminPassword = arg.replace(adminPasswordArg, "");
                adminPassword = SystemUtil.sanitize(adminPassword);
            }
            if (arg.startsWith(detailsModeArg)) {
                detailsMode = Boolean.parseBoolean(arg.replace(detailsModeArg, ""));
            }
        }

        logger.info(String.format("Reading configuration from path=%s", configPath));
        final ConfigWrapper config = new ConfigWrapper(ConfigFactory.loadFromFile(configPath));


        if (config.isAuthenticationActivated()) {
            logger.info("Check authorized user DB version ...");
            if (!AuthorizedUserRepository.getInstance().checkAlterAndMigrateAuthorizedUser()) {
                logger.fatal("Failed to start server.");
                return;
            }
            logger.info("Done.");
        }

        // db init and updates checks (e.g. cleanup cards db on new version)
        RepositoryUtil.bootstrapLocalDb();
        logger.info("Done.");

        logger.info("Loading extension packages...");
        if (!extensionFolder.exists()) {
            if (!extensionFolder.mkdirs()) {
                logger.error("Could not create extensions directory.");
            }
        }
        File[] extensionDirectories = extensionFolder.listFiles();
        List<ExtensionPackage> extensions = new ArrayList<>();
        if (extensionDirectories != null) {
            for (File f : extensionDirectories) {
                if (f.isDirectory()) {
                    try {
                        logger.info(" - Loading extension from " + f);
                        extensions.add(ExtensionPackageLoader.loadExtension(f));
                    } catch (IOException e) {
                        logger.error("Could not load extension in " + f + '!', e);
                    }
                }
            }
        }
        logger.info("Done.");

        if (!extensions.isEmpty()) {
            logger.info("Registering custom sets...");
            for (ExtensionPackage pkg : extensions) {
                for (ExpansionSet set : pkg.getSets()) {
                    logger.info("- Loading " + set.getName() + " (" + set.getCode() + ')');
                    Sets.getInstance().addSet(set);
                }
                PluginClassloaderRegistery.registerPluginClassloader(pkg.getClassLoader());
            }
            logger.info("Done.");
        }

        logger.info("Loading cards...");
        CardScanner.scan();
        logger.info("Done.");

        // cards preload with ratings
        if (RateCard.PRELOAD_CARD_RATINGS_ON_STARTUP) {
            RateCard.bootstrapCardsAndRatings();
            logger.info("Done.");
        }

        if (testMode) {
             logger.info("Skipping user stats update in test mode");
        } else {
            try {
                logger.info("Updating user stats DB...");
                UserStatsRepository.instance.updateUserStats();
                logger.info("Done.");
            } catch (Exception e) {
                logger.warn("Failed to update user stats: " + e.getMessage());
            }
        }
        deleteSavedGames();

        int gameTypes = 0;
        for (GamePlugin plugin : config.getGameTypes()) {
            gameTypes++;
            GameFactory.instance.addGameType(plugin.getName(), PluginUtil.loadGameType(plugin), PluginUtil.loadPlugin(plugin, plugin.getClassName()));
        }

        int tourneyTypes = 0;
        for (GamePlugin plugin : config.getTournamentTypes()) {
            tourneyTypes++;
            TournamentFactory.instance.addTournamentType(plugin.getName(), PluginUtil.loadTournamentType(plugin), PluginUtil.loadPlugin(plugin, plugin.getClassName()));
        }

        int playerTypes = 0;
        for (Plugin plugin : config.getPlayerTypes()) {
            playerTypes++;
            PlayerFactory.instance.addPlayerType(plugin.getName(), PluginUtil.loadPlugin(plugin, plugin.getClassName()));
        }

        int cubeTypes = 0;
        for (Plugin plugin : config.getDraftCubes()) {
            cubeTypes++;
            CubeFactory.instance.addDraftCube(plugin.getName(), PluginUtil.loadPlugin(plugin, plugin.getClassName()));
        }

        int deckTypes = 0;
        for (Plugin plugin : config.getDeckTypes()) {
            deckTypes++;
            DeckValidatorFactory.instance.addDeckType(plugin.getName(), PluginUtil.loadPlugin(plugin, plugin.getClassName()));
        }

        for (ExtensionPackage pkg : extensions) {
            for (Map.Entry<String, Class> entry : pkg.getDraftCubes().entrySet()) {
                logger.info("Loading extension: [" + entry.getKey() + "] " + entry.getValue().toString());
                cubeTypes++;
                CubeFactory.instance.addDraftCube(entry.getKey(), entry.getValue());
            }
            for (Map.Entry<String, Class> entry : pkg.getDeckTypes().entrySet()) {
                logger.info("Loading extension: [" + entry.getKey() + "] " + entry.getValue().toString());
                deckTypes++;
                DeckValidatorFactory.instance.addDeckType(entry.getKey(), entry.getValue());
            }
        }

        logger.info("Config - server address:   " + config.getServerAddress());
        logger.info("Config - server port:      " + config.getPort());
        logger.info("Config - max game threads: " + config.getMaxGameThreads());
        logger.info("Config - max seconds idle: " + config.getMaxSecondsIdle());
        logger.info("Config - max AI opponents: " + config.getMaxAiOpponents());
        logger.info("Config - min usr name le.: " + config.getMinUserNameLength());
        logger.info("Config - max usr name le.: " + config.getMaxUserNameLength());
        logger.info("Config - min pswrd length: " + config.getMinPasswordLength());
        logger.info("Config - max pswrd length: " + config.getMaxPasswordLength());
        logger.info("Config - inv.usr name pat: " + config.getInvalidUserNamePattern());
        logger.info("Config - save game active: " + (config.isSaveGameActivated() ? "true" : "false"));
        logger.info("Config - backlog size    : " + config.getBacklogSize());
        logger.info("Config - lease period    : " + config.getLeasePeriod());
        logger.info("Config - sock wrt timeout: " + config.getSocketWriteTimeout());
        logger.info("Config - max pool size   : " + config.getMaxPoolSize());
        logger.info("Config - num accp.threads: " + config.getNumAcceptThreads());
        logger.info("Config - second.bind port: " + config.getSecondaryBindPort());
        logger.info("Config - users registr.:   " + (config.isAuthenticationActivated() ? "true" : "false"));
        logger.info("Config - users anon:       " + (!config.isAuthenticationActivated() ? "true" : "false"));
        logger.info("Config - mailgun api key : " + (config.getMailgunApiKey() == null || config.getMailgunApiKey().isEmpty() ? "not set" : "set"));
        logger.info("Config - mailgun domain  : " + config.getMailgunDomain());
        logger.info("Config - mail smtp Host  : " + config.getMailSmtpHost());
        logger.info("Config - mail smtpPort   : " + config.getMailSmtpPort());
        logger.info("Config - mail user       : " + config.getMailUser());
        logger.info("Config - mail passw. len.: " + config.getMailPassword().length());
        logger.info("Config - mail from addre.: " + config.getMailFromAddress());
        logger.info("Config - google account  : " + config.getGoogleAccount());

        logger.info("Loaded game types: " + gameTypes
                + ", tourneys: " + tourneyTypes
                + ", players: " + playerTypes
                + ", cubes: " + cubeTypes
                + ", decks: " + deckTypes);
        if (gameTypes == 0) {
            logger.error("ERROR, can't load any game types. Check your config.xml in server's config folder (example: old jar versions after update).");
        }

        final ManagerFactory managerFactory = new MainManagerFactory(config);
        try {
            MageServerImpl mageServerImpl = new MageServerImpl(managerFactory, adminPassword, testMode, detailsMode);
            if (Boolean.getBoolean(remotingProp)) {
                logger.warn(remotingProp + "=true is ignored: desktop clients are no longer supported, web clients only");
            }

            // WebSocket Server
            int websocketPort = config.getWebsocketPort();
            mage.server.websocket.WebSocketServerImpl webSocketServer = new mage.server.websocket.WebSocketServerImpl(
                    new java.net.InetSocketAddress(websocketPort), mageServerImpl, managerFactory, config.getWebsocketAllowedOrigins(),
                    adminPassword);
            webSocketServer.start();
            webSocketServer.awaitStartup(10, java.util.concurrent.TimeUnit.SECONDS);

            Runtime.getRuntime().addShutdownHook(new Thread(webSocketServer::shutdown, "WebSocket shutdown"));
            logger.info("Started MAGE WebSocket server - listening on port " + websocketPort);
            // before Health: the first JDK HTTP server fixes the request time limits for both
            mage.server.support.SupportHooks.start();
            Health.start(managerFactory);
            BackupStatus.startChecks();
            // AI-against-AI batches carry on from before a restart
            mage.server.solo.BatchRunner.get().start();
            // Career tables from before a restart still pay when their saved games resume (B270)
            if (mage.server.career.CareerService.enabled(config.isAuthenticationActivated())) {
                int restored = mage.server.career.CareerService.get().restoreTables();
                if (restored > 0) {
                    logger.info("Career: " + restored + " tables kept from before the restart");
                }
                // games played before progression counted them (B308), once per server
                mage.server.career.CareerHistoryCredit.startOnce();
            }
            UpdateRunner.startWatching();

            if (testMode) {
                logger.info("MAGE server running in test mode");
            }
            initStatistics();
            if (Boolean.getBoolean(exitAfterStartupProp)) {
                logger.info("Started; stopping now (" + exitAfterStartupProp + ")");
                System.exit(0);
            }
        } catch (Exception ex) {
            logger.fatal("Failed to start server", ex);
        }
    }

    static void initStatistics() {
        ServerMessagesUtil.instance.setStartDate(System.currentTimeMillis());
    }

    private static void deleteSavedGames() {
        File directory = new File("saved/");
        if (!directory.exists()) {
            directory.mkdirs();
        }
        File[] files = directory.listFiles(
                (dir, name) -> name.endsWith(".game")
        );
        if (files != null) {
            for (File file : files) {
                file.delete();
            }
        }
    }

    public static MageVersion getVersion() {
        return version;
    }

    public static boolean isTestMode() {
        return testMode;
    }
}

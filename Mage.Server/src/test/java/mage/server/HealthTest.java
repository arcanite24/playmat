package mage.server;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import static org.assertj.core.api.Assertions.assertThat;

/** The loopback endpoint: /healthz, and the deploy's drain (start and cancel maintenance), against a stand-in server. */
public class HealthTest {

    private final List<String> calls = new ArrayList<>();
    private boolean planned;
    private boolean failing;
    private HttpServer server;

    @BeforeEach
    void listen() throws IOException {
        server = Health.listen(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), new Health.Operations() {
            @Override
            public Map<String, Object> snapshot() {
                if (failing) {
                    throw new IllegalStateException("broken");
                }
                Map<String, Object> health = new LinkedHashMap<>();
                health.put("status", "ok");
                health.put("gamesRunning", 2);
                return health;
            }

            @Override
            public long startMaintenance(long minutes, String message) {
                calls.add("start " + minutes + " " + message);
                planned = true;
                return 1234L;
            }

            @Override
            public boolean cancelMaintenance() {
                calls.add("cancel");
                boolean was = planned;
                planned = false;
                return was;
            }
        });
    }

    @AfterEach
    void stop() {
        server.stop(0);
    }

    @Test
    void healthzAnswersWithTheSnapshot() throws Exception {
        Answer ok = request("GET", "/healthz");
        assertThat(ok.status).isEqualTo(200);
        assertThat(ok.body).contains("\"status\":\"ok\"").contains("\"gamesRunning\":2");
        failing = true;
        Answer broken = request("GET", "/healthz");
        assertThat(broken.status).isEqualTo(500);
        assertThat(broken.body).contains("\"status\":\"error\"");
    }

    @Test
    void maintenanceStartsAndIsCalledOff() throws Exception {
        Answer started = request("POST", "/maintenance?minutes=15&message=Updating%20to%20v1.2.3");
        assertThat(started.status).isEqualTo(200);
        assertThat(started.body).contains("\"maintenanceAt\":1234");
        assertThat(calls).containsExactly("start 15 Updating to v1.2.3");

        Answer cancelled = request("POST", "/maintenance/cancel");
        assertThat(cancelled.status).isEqualTo(200);
        assertThat(cancelled.body).contains("\"cancelled\":true");
        assertThat(request("POST", "/maintenance/cancel").body).as("nothing left to call off").contains("\"cancelled\":false");
    }

    @Test
    void maintenanceRefusesWhatItCantDo() throws Exception {
        assertThat(request("GET", "/maintenance?minutes=15").status).as("looking changes nothing").isEqualTo(405);
        assertThat(request("GET", "/maintenance/cancel").status).isEqualTo(405);
        assertThat(request("POST", "/maintenance").status).as("no minutes").isEqualTo(400);
        assertThat(request("POST", "/maintenance?minutes=soon").status).isEqualTo(400);
        assertThat(request("POST", "/maintenance?minutes=-1").status).isEqualTo(400);
        assertThat(request("POST", "/maintenance?minutes=1441").status).as("a day at most").isEqualTo(400);
        StringBuilder longMessage = new StringBuilder();
        for (int i = 0; i < 301; i++) {
            longMessage.append('x');
        }
        assertThat(request("POST", "/maintenance?minutes=5&message=" + longMessage).status).isEqualTo(400);
        assertThat(request("POST", "/maintenance/start").status).isEqualTo(404);
        assertThat(calls).isEmpty();
    }

    @Test
    void readsTheQueryString() {
        assertThat(Health.query(null)).isEmpty();
        assertThat(Health.query("minutes=0&message=a+b%26c&minutes=9&flag"))
                .containsEntry("minutes", "0")
                .containsEntry("message", "a b&c")
                .containsEntry("flag", "");
    }

    private Answer request(String method, String path) throws IOException {
        URL url = new URL("http://127.0.0.1:" + server.getAddress().getPort() + path);
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setRequestMethod(method);
        if ("POST".equals(method)) {
            connection.setDoOutput(true);
            connection.getOutputStream().close();
        }
        int status = connection.getResponseCode();
        InputStream in = status < 400 ? connection.getInputStream() : connection.getErrorStream();
        ByteArrayOutputStream body = new ByteArrayOutputStream();
        if (in != null) {
            byte[] buffer = new byte[4096];
            for (int read; (read = in.read(buffer)) > 0; ) {
                body.write(buffer, 0, read);
            }
            in.close();
        }
        connection.disconnect();
        return new Answer(status, new String(body.toByteArray(), StandardCharsets.UTF_8));
    }

    private static final class Answer {
        final int status;
        final String body;

        Answer(int status, String body) {
            this.status = status;
            this.body = body;
        }
    }
}

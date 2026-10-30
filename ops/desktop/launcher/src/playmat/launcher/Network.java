package playmat.launcher;

import java.io.IOException;
import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.ServerSocket;
import java.net.SocketException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;

/** This computer's addresses, for the "friends can join at" line and the server's allowed origins. */
final class Network {

    private Network() {
    }

    /** IPv4 addresses other devices on the local network can probably reach, best guess first. */
    static List<String> lanAddresses() {
        List<String> preferred = new ArrayList<>();
        List<String> others = new ArrayList<>();
        try {
            for (NetworkInterface nic : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                if (!nic.isUp() || nic.isLoopback() || nic.isVirtual() || nic.isPointToPoint()) {
                    continue;
                }
                String name = (nic.getName() + " " + nic.getDisplayName()).toLowerCase(Locale.ENGLISH);
                // Docker, WSL, VirtualBox, VMware and VPN adapters aren't where friends are
                boolean virtual = name.matches(".*(docker|veth|br-|vbox|virtualbox|vmware|vmnet|hyper-v|vethernet|wsl|tun|tap|utun|zerotier|tailscale|wireguard).*");
                for (InetAddress address : Collections.list(nic.getInetAddresses())) {
                    if (address instanceof Inet4Address && !address.isLoopbackAddress() && !address.isLinkLocalAddress()) {
                        String ip = address.getHostAddress();
                        if (!virtual && address.isSiteLocalAddress()) {
                            preferred.add(ip);
                        } else {
                            others.add(ip);
                        }
                    }
                }
            }
        } catch (SocketException e) {
            Log.warn("Can't list network interfaces: " + e.getMessage());
        }
        preferred.addAll(others);
        return preferred;
    }

    /** Browser origins of the pages this launcher serves. */
    static List<String> origins(int port) {
        Set<String> hosts = new LinkedHashSet<>();
        hosts.add("localhost");
        hosts.add("127.0.0.1");
        hosts.addAll(lanAddresses());
        try {
            String host = InetAddress.getLocalHost().getHostName().toLowerCase(Locale.ENGLISH);
            if (!host.isEmpty() && !host.equals("localhost")) {
                hosts.add(host);
                if (!host.contains(".")) {
                    hosts.add(host + ".local");
                    hosts.add(host + ".lan");
                    hosts.add(host + ".home");
                }
            }
        } catch (IOException e) {
            // no host name: addresses only
        }
        List<String> origins = new ArrayList<>();
        for (String host : hosts) {
            origins.add("http://" + host + ":" + port);
        }
        // a public name in front of this launcher (a tunnel or reverse proxy), e.g.
        // PLAYMAT_EXTRA_ORIGINS="https://play.example.com"; several separated by commas or spaces
        String extra = System.getenv("PLAYMAT_EXTRA_ORIGINS");
        if (extra != null) {
            for (String origin : extra.split("[,\\s]+")) {
                if (!origin.isEmpty() && !origins.contains(origin)) origins.add(origin);
            }
        }
        return origins;
    }

    /** True when nothing else listens on the port (on any interface). */
    static boolean isFree(int port) {
        try (ServerSocket socket = new ServerSocket()) {
            // connections of a server that just stopped may linger in TIME_WAIT; they don't block a new listener
            socket.setReuseAddress(true);
            socket.bind(new java.net.InetSocketAddress(port));
            return true;
        } catch (IOException e) {
            return false;
        }
    }
}

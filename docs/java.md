# Java runtimes

Minecraft servers need a Java version that matches the game version. CraftPanel downloads the right
runtime by itself, so the machine needs no Java installed beforehand.

## Which Java a server gets

Unless a server is pinned to a version, the panel derives the Java major version from the
Minecraft version:

| Minecraft version | Java |
|---|---|
| 1.16.5 and older | 8 |
| 1.17 to 1.19.x | 17 |
| 1.20 and newer 1.x releases | 21 |
| 26.1 and newer (year-based version numbers) | 25 |

The panel downloads exactly these four majors: 8, 17, 21 and 25.

When a server starts, the panel looks for a runtime of exactly that major, in this order:

1. a runtime the panel downloaded, in `<data_dir>/runtimes/java-<major>/`;
2. a runtime installed on the system (see [Using a system Java](#using-a-system-java));
3. if **Fetch Java by itself** is on: the panel starts downloading it, and the start is refused
   with "the panel is fetching it now; start again once that run is done";
4. if fetching is off: a newer Java up to the next long-term release (8 may be replaced by 11,
   17 by 21, 21 by 25), if one is installed;
5. otherwise the start is refused, naming the missing version and the versions that are present.

With fetching on, creating a server also downloads its runtime as part of the setup, so a new
server is usually ready to start immediately.

To use a different Java for one server, choose it in the server's startup settings. Only runtimes
present on the machine are offered; **Automatic** uses the table above.

## Where runtimes come from

Runtimes are **Eclipse Temurin** JREs from [Adoptium](https://adoptium.net), for Linux on x86_64
or aarch64:

- The panel asks `api.adoptium.net` for the latest release of the major version.
- The archive is downloaded only from Adoptium and GitHub, and verified against the SHA-256
  checksum Adoptium publishes for it. A release without a checksum is not used, and an archive that
  does not match is discarded.
- It is unpacked into a staging directory and moved into place only when complete, so a failed
  download never leaves a half-installed runtime.
- The result is stored in `<data_dir>/runtimes/java-<major>/`, readable and executable by the game
  accounts.

A runtime is downloaded once. The panel never updates a runtime in the background; replacing it is
always an administrator's decision (see below). On other architectures nothing can be downloaded,
and the admin page says so.

## Managing runtimes

**Administration → Java runtimes** shows one row per Java major:

- whether the panel downloaded it, whether the system has one of its own, or neither;
- version, size, download date and directory;
- how many servers need it, and which of them are running on it now.

| Button | Effect |
|---|---|
| **Fetch it** | Download the runtime. |
| **Fetch again** | Download the newest build of that major and replace the existing one, for example after a Java security release. The old one is removed only after the new one is in place. |
| **Remove** | Delete the runtime the panel downloaded. |

**Fetch again** and **Remove** are disabled while a server that uses that major is running: a JVM
whose files are replaced or deleted crashes later, at an unpredictable moment. Stop those servers
first.

These buttons work even when automatic fetching is switched off.

## Switching automatic fetching off

**Administration → Panel settings → Fetch Java by itself** is on by default. Turn it off on a
machine without outbound internet access, or when you install Java with the package manager.

With fetching off, a server whose Java version is missing refuses to start and says which version
it needs. Runtimes already downloaded stay and keep working, and a newer Java may stand in as
described above.

With fetching on, a missing runtime is always downloaded rather than replaced by a stand-in. On a
machine that cannot reach Adoptium, starts then fail with a download error; turn fetching off
there.

## Using a system Java

The panel also uses Java installations it finds on the machine:

- `$JAVA_HOME` of the panel service;
- directories under `/usr/lib/jvm`, `/usr/java`, `/opt/java` and `/opt/jdk`;
- the `java` found on the service's `PATH`.

Each installation is identified by its `release` file. For example, on Debian or Ubuntu:

```bash
sudo apt install openjdk-21-jre-headless
```

The game accounts must be able to read and execute the installation; packages from the
distribution are.

### Offline installation

On a machine without internet access, unpack a Temurin JRE archive by hand so that `bin/java` and
`release` lie directly under `<data_dir>/runtimes/java-<major>/`:

```bash
sudo mkdir -p /var/lib/craftpanel/runtimes/java-21
sudo tar -xzf OpenJDK21U-jre_x64_linux_hotspot_*.tar.gz \
     -C /var/lib/craftpanel/runtimes/java-21 --strip-components=1
sudo chown -R craftpanel:craftpanel /var/lib/craftpanel/runtimes/java-21
sudo chmod -R a+rX /var/lib/craftpanel/runtimes/java-21
```

The panel treats it like a runtime it downloaded.

## Troubleshooting

| Message | Cause |
|---|---|
| "The panel is fetching it now" | The runtime is being downloaded. Start the server again when the operation has finished. |
| Java download failed | Adoptium could not be reached or the download broke off. Check outbound HTTPS to `api.adoptium.net` and `github.com`, or switch fetching off and install Java yourself. |
| Java runtime missing | Fetching is off and no suitable runtime is installed. Install one, fetch it on the admin page, or turn fetching on. |
| Runtime in use | **Fetch again** or **Remove** was refused because a server runs on that runtime. Stop it first. |

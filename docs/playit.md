# Public addresses with playit.gg

Players outside your network can reach a server without port forwarding through a
[playit.gg](https://playit.gg) tunnel. Each panel user connects **their own** playit.gg account;
the panel provides none and the operator sets nothing up.

playit.gg is optional. If nobody connects an account, nothing calls playit.gg and servers are
reached through the machine's own address and port as usual.

## How it works

- Each panel user links their playit.gg account once, on their account page.
- A server's owner requests a public address for the server. The panel creates a tunnel on the
  owner's playit.gg account that forwards to the server's local port.
- One playit.gg agent process runs per connected user, and only while that user has at least one
  tunnel. It runs as the unprivileged `craftpanel` account and restarts with the panel; connected
  players see a short interruption.
- The agent binary (playit agent `v1.0.10`) is downloaded from playit.gg's GitHub releases on
  first use into `<data_dir>/cache/playit/` and checked against a SHA-256 checksum built into the
  panel. It is available for x86_64, aarch64, armv7 and i686.
- Each user's agent key is stored in `<data_dir>/playit/<user-id>/`, readable only by the panel.

## Limits

| playit.gg plan | Public addresses at the same time |
|---|---|
| Free | 4 |
| Premium | 16 |

The limit belongs to each user's playit.gg account, not to the panel: every connected user has
their own four (or sixteen). A Minecraft Java server uses one. When all are in use, requesting
another address is refused until a server gives its address back.

Free tunnels use playit.gg's global anycast network; the panel does not offer a region choice.

## Connecting an account

1. Open your account page and find **Public addresses for your servers**.
2. Press **Connect to playit.gg**. The panel shows a link to playit.gg.
3. Open the link, sign in to playit.gg or create an account there, and confirm. No password is typed
   into the panel.
4. The panel notices the confirmation by itself. The link is valid for 15 minutes; press
   **New link** if it runs out.

The account section then shows the tunnel service state, the playit.gg account state and how many
addresses are in use. A playit.gg *guest* account (created without an e-mail address) can be lost;
add an e-mail address at playit.gg to keep it.

## Giving a server a public address

On the server's overview page, the **Public address** block offers **Request public address**. Only
the server's owner and panel administrators can request one, and the address always lives on the
owner's playit.gg account. If the owner has not connected an account, the owner sees
**Set up playit.gg**, which leads to the account page.

When the tunnel is ready, the address is shown on the overview page and in the server header.
Players enter that address in their Minecraft client. playit.gg may report several addresses; use
the first one shown. The others are listed but may not work, because playit.gg routes connections
by the name in the address.

**Give the address back** deletes the tunnel and frees the slot.

## Disconnecting

On the account page, **Disconnect** offers to keep or delete the addresses at playit.gg. The key
is removed from the panel either way. Administrators see every connected account under
**Administration → Public addresses (all users)** and can disconnect any of them.

When **Outbound services** is switched off in the panel settings, connecting, requesting and
deleting addresses is refused. Disconnecting while keeping the addresses still works.

## Troubleshooting

| Symptom | What to do |
|---|---|
| "The address could not be set up" or "not carrying" | The block shows playit.gg's reason. Fix it at playit.gg if needed and press **Set up again**. |
| "The address was removed at playit.gg" | The tunnel was deleted in the playit.gg dashboard. The panel does not recreate it on its own; press **Set up again**. |
| The claim is no longer valid | The agent was removed at playit.gg or the account deleted. Disconnect, then connect again. |
| "This agent may not manage its own tunnels" | Disconnect and connect again. |
| The tunnel service is not running | It restarts automatically and gives up after five failures in a row. Use **Restart tunnel service** on the account page and check `journalctl -u craftpanel`. |
| The program file failed its checksum | The download was damaged or replaced. The file is deleted; check the machine's connection to GitHub and restart the tunnel service. |
| Players get "Unknown host" | Some ISP resolvers do not resolve playit.gg addresses. The player can use a different DNS server; playit.gg has a help page on this. |
| playit.gg is unreachable | The last known address stays shown and the panel retries with growing intervals, up to five minutes. The server itself keeps running and stays reachable locally. |

The agent needs outbound access to `api.playit.gg` and to playit.gg's tunnel network. A failing
tunnel never affects the Minecraft server itself.

# Circulo — Remote access (proposal)

- **Status:** Proposal (2026-11-21). Nothing here is implemented yet.
- **Related:** [TRD](trd.md) · [PRD](prd.md) · [Flow](flow.md)

## 1. What we need

Remote and mobile are post-v1.0, but they shape the transport now. The requirements:

- A phone or another Mac reaches **this** daemon, with the project files, Git and provider sessions it owns.
- No account and no cloud of our own. The product stays local-first.
- The daemon keeps binding loopback. Remote is a tunnel in front of it, not a new surface.
- The protocol does not change: `wss://` carries the same messages, and the token remains the application-level gate.

The daemon and the client already satisfy the last two: the protocol is transport-agnostic, the client accepts any URL, and the handshake requires the token. What is missing is how the connection gets from the phone to `127.0.0.1`.

## 2. Tailcat

[Tailcat](https://tailscale.com/tailcat) is Tailscale's data plane without its control plane: "netcat over Tailscale's data plane". One side runs a listener and prints a short address; the other connects with that address. Traffic is WireGuard end-to-end, peer to peer when NAT allows it and through DERP relays when it does not. No account, no root, no route or DNS changes.

What matters for us:

| | |
|---|---|
| **Fit with the product** | Strong. No account, no tailnet, no OAuth, ephemeral by default. It is the same philosophy as Circulo. |
| **What it changes** | Nothing in the daemon. `tailcat serve <port>` forwards a port to `127.0.0.1`, and the existing `ws://` + token works through the tunnel. WireGuard gives confidentiality in transit. |
| **Mobile** | The project ships `TailcatKit`, a Swift 6 package with async/await clients and TCP connections for macOS 14+ and iOS 17+. A native iOS client could embed it instead of shelling out. |
| **Persistence** | Ephemeral keys by default: the address dies with the process, which is great for one-off pairing and bad for "my phone reconnects tomorrow". `tailcat genkey` saves a key so the address is stable, and `serve --allow=<nodekey>` restricts which clients may connect. |
| **Security model** | The address is a bearer capability (it embeds a WireGuard pre-shared key). Sharing it is equivalent to sharing access. For a stable setup: saved key + `--allow` + our token handshake. Never publish the address. |
| **Risks** | Tailcat has no API, CLI or wire stability promises, and the public DERP relays are rate-limited with no SLA. Direct paths are the normal case; a relayed path is slower. Pin a version and keep it at the edge of our design. |

For a phone that pairs today and reconnects next week, the flow would be: a saved key, the address stored with the pairing, `--allow` for the client node key, and the daemon token as the second gate.

## 3. A Tailscale tailnet

The alternative is the conventional one: both devices join a tailnet, and `tailscale serve` exposes the loopback daemon over HTTPS with an automatic certificate, MagicDNS name and ACLs. `wss://` then works directly.

| | |
|---|---|
| **Strengths** | Stable DNS name, device identity, ACLs, certificates, no address to keep secret. |
| **Costs** | A Tailscale account and a third-party control plane in the path — the thing "no cloud of its own" pushes against. Every device must join the tailnet. |
| **Fit** | Good for teams and for people who already run a tailnet; wrong as the default story for a local-first app. |

`tailscale serve` proxies to `http://127.0.0.1:<port>`. A WebSocket upgrade through it needs a smoke test; if the proxy does not forward the upgrade, `tailscale serve --tls-terminated-tcp=<port> tcp://127.0.0.1:<port>` forwards raw TCP with TLS terminated by Tailscale, which is transport-agnostic.

## 4. Bring your own TLS

A user with a VPS, a reverse proxy or a private network can terminate TLS in front of the daemon and point the client at `wss://host`. Nothing to build: the client already speaks `wss://` and checks certificates against the webpki roots. This stays supported as the escape hatch.

## 5. Recommendation

1. **Keep the daemon loopback-only and the token mandatory.** Remote is a tunnel; the protocol and the daemon stay as they are.
2. **Make tailcat the recommended path**, because it keeps the product account-free and it has a native path to iOS. Pairing would: start `tailcat serve` for the daemon port, show the address (a QR code is the natural pairing surface), and store it with the saved key and the `--allow` node key.
3. **Keep `wss://` working** for tailnets, reverse proxies and LAN use. No code change.
4. **Treat the address as a secret.** Pairing UI must say so, and the stable setup must use a saved key plus `--allow`.
5. **Isolate tailcat at the edge**: a supervisor that starts and stops the tunnel process with the daemon, and a client that only knows a URL. If tailcat's format changes, nothing inside Circulo does.

## 6. Open questions

- Pairing UX: QR code, copyable address, or both. Does the phone keep the address, or re-pair each time?
- Where does the tunnel process live: inside the desktop app, inside a future daemon setting, or as a documented `tailcat` the user runs?
- Do we embed `TailcatKit` in an iOS client, or start with a web client and revisit native?
- Does the tailcat CLI's lifetime match the daemon's? A tunnel that outlives the daemon is a dangling listener; one that dies early drops the phone.
- If a relayed path is the only one, is the throughput acceptable for a long transcript? Measure before promising.

## 7. Sources

- Tailcat: <https://tailscale.com/tailcat> and <https://github.com/tailscale/tailcat> (README, security notes, Swift bindings).
- Tailscale Serve: <https://tailscale.com/kb/1242/tailscale-serve>.

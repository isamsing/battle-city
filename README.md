# Battle City

A reimagined 2D version of the NES Battle City, built in Rust with [Bevy 0.18](https://bevyengine.org/).
Play alone, or **up to 4 players online in Chrome** with rollback netcode (GGRS over WebRTC via matchbox).

## Controls

| Action  | Keyboard        | Gamepad                |
|---------|-----------------|------------------------|
| Move    | Arrow keys / WASD | D-pad or left stick  |
| Fire    | Space           | A / X                  |
| Ability | Left Shift      | B / Y                  |
| Leave the match | Escape   |                        |
| Debug overlay (FPS, ping, desync) | F3 |                  |

Online matches cannot be paused: a pause on one machine would desync everyone, so Escape leaves the match.
The Left Shift / B button input is sent over the network but is not used by the rules yet.

## Requirements

- Rust (stable) with the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/) and [binaryen](https://github.com/WebAssembly/binaryen) (`wasm-opt`) for the web build
- Linux native builds need the Bevy system libraries:
  `sudo apt-get install libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`

## Run

```bash
cargo run --release          # native
trunk serve --release        # web: http://127.0.0.1:8080/battle-city/
trunk build --release        # production bundle in dist/
```

The release bundle is about 19 MB raw / 6.5 MB gzipped. CI fails if the gzipped wasm exceeds its budget.

## How the game is built

- `src/sim/` is the whole game: a deterministic, Bevy-free simulation (integer maths, seeded RNG, fixed
  iteration order). One `Sim::step` is used offline and online.
- `src/scenes/level/` drives the sim from GGRS inputs, renders it, and draws the HUD and debug overlay.
- `src/net/` holds the matchbox socket, lobby protocol, GGRS session and stats.
- Offline play runs through a GGRS sync-test session, so rollback self-checks run in debug builds.

## Play online (1 to 4 players)

1. Everyone: start the game, choose **MultiPlayer**.
2. **Host**: press `H`. A 4-character room code appears. Press `1`-`4` to set how many players.
3. **Others**: press `J`, type the code, press Enter. Press `R` to mark yourself ready.
4. The lobby lists each player with their tank colour (P1 yellow, P2 green, P3 blue, P4 red) and ready state.
5. When the roster is full and everyone is ready, the host presses Enter to start.

Online play is co-op against the enemy waves with one shared base. Friendly fire stuns a teammate. If a
player drops, their tank is taken over by the AI on every peer and the match continues.

## Signaling server, TURN and deployment (Fly.io)

One Fly app runs the [matchbox](https://github.com/johanhelsing/matchbox) signaling server and a coturn relay
(`Dockerfile`, `start.sh`, `turnserver.conf`, `fly.toml`).

```bash
fly launch --no-deploy          # first time only; keeps fly.toml
fly ips allocate-v4             # dedicated IPv4 needed for TURN
# put the address into start.sh (PUBLIC_IP)
fly secrets set TURN_USER=battlecity TURN_PASS=<long random string>
fly deploy
```

Point the game at your server and TURN credentials at **build time**:

```bash
BATTLE_CITY_SERVER=wss://your-app.fly.dev \
BATTLE_CITY_TURN_HOST=your-app.fly.dev \
BATTLE_CITY_TURN_USER=battlecity BATTLE_CITY_TURN_PASS=<same string> \
  trunk build --release
```

You can also change the signaling server in the lobby with `S`. Clients use Google STUN first and fall back to
TURN over UDP, then TCP, so players behind strict NATs can still connect. TURN credentials end up in the
client bundle, so use a dedicated low-privilege account and rotate it if it leaks.

For local testing run `cargo install matchbox_server && matchbox_server` and set the server to
`ws://127.0.0.1:3536`.

## Tests

```bash
cargo test                                                   # unit, determinism and netplay tests
cargo test --release --test netplay_sim_tests -- --ignored  # 10-minute, 4-peer, 100 ms soak
```

- `tests/sim_determinism_tests.rs` runs the same inputs twice and checks identical state every frame, plus
  rollback replay, AI takeover and the friendly-fire stun.
- `tests/netplay_sim_tests.rs` runs real GGRS sessions for 2 and 4 peers over a simulated network with 100 ms
  latency, jitter and packet loss, and checks that no desync is reported and confirmed frames match.

## CI

`.github/workflows/ci.yml` builds and tests the native target and builds the release WASM bundle on every
pull request. Version tags (`v*`) deploy the web build to GitHub Pages and the servers to Fly.io.

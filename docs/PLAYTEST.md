# Manual acceptance session (4 players, 4 networks)

Automated tests cover the simulation, rollback and a simulated lossy network. They cannot cover real GPUs,
real NATs or real people. Run this once before calling the feature done.

## Setup

1. Deploy the signaling server and TURN relay (see README, "Signaling server, TURN and deployment").
2. Build the client against it and host the `dist/` folder anywhere reachable by all four players:
   ```bash
   BATTLE_CITY_SERVER=wss://YOUR-APP.fly.dev BATTLE_CITY_TURN_HOST=YOUR-APP.fly.dev \
   BATTLE_CITY_TURN_USER=... BATTLE_CITY_TURN_PASS=... trunk build --release
   ```
3. Four players, each on a **different network**, open the page in current desktop Chrome.
   At least one player should be behind a strict NAT or on mobile data, to exercise TURN.

## Run

1. Everyone: MultiPlayer. Host: `H`, then `4`. Others: `J`, type the code, Enter, then `R`.
2. Host presses Enter when the roster shows 4 ready players.
3. Everyone presses `F3` to show the debug overlay.
4. Play one full stage together, then keep playing to about 10 minutes of total session time.

## What to record

| Check | Pass condition | Where to look |
|-------|----------------|---------------|
| Join by code | All four appear in the lobby with their colours (yellow, green, blue, red) | Lobby list |
| Connection path | Note whether each player connected directly or through TURN | `chrome://webrtc-internals` |
| Frame rate | Steady 60 FPS on every machine | F3 overlay, `FPS` line |
| Ping | Per-peer round trip, ideally under about 150 ms | F3 overlay, `ping` line |
| Desyncs | Stays at `in sync` for the whole session | F3 overlay, `desync` line |
| Stage finish | All enemies cleared, `STAGE CLEAR`, then stage 2 starts | Screen |
| Load time | Page interactive in under about 5 s on broadband | Stopwatch / DevTools Network tab |
| Audio | Music starts after the first click or key press | Ears |
| Tab focus | Switching tabs and back does not leave the tank driving | Screen |
| Drop handling | One player closes the tab: the rest keep playing, the HUD shows `[AI]` for that slot | Screen |

## If something goes wrong

- A desync shows `DESYNC` in the overlay and in the browser console with the frame number. Save the console
  log from every player and the frame number; with those the failing frame can be replayed from the inputs.
- Chrome DevTools network throttling does not reliably shape WebRTC traffic. To test ~100 ms latency, use an
  OS-level tool such as `tc netem` or a network link conditioner.
- In the browser console, `window.__bc` holds the current lobby and match state (frame, checksum, desync count).

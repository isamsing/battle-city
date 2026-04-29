# Battle City

A reimagined 2D version of the NES game Battle City, built in Rust with [Bevy 0.18](https://bevyengine.org/).

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.95+

## Build & Run

```bash
# Debug build and run
cargo run

# Release build (optimized)
cargo build --release
./target/release/battle-city
```

## Controls

| Action | Key |
|--------|-----|
| Move | Arrow keys |
| Fire | Space |
| Ability | Left Shift |
| Pause | Escape |

## Multiplayer

P2P multiplayer uses rollback netcode (GGRS) over WebRTC (matchbox).

### Setup

1. Install and run the signaling server:

```bash
cargo install matchbox_server
matchbox_server
```

2. Run two game instances (in separate terminals):

```bash
cargo run  # Terminal 1
cargo run  # Terminal 2
```

### Connecting

1. Both players: Press Enter → select **2 PLAYERS** → enter Lobby
2. **Host**: Press `H` — a 4-character room code is displayed
3. **Joiner**: Press `J` → type the room code → press Enter
4. Once connected, both players enter the game

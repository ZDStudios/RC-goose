# ADR 0059: Opt-in remote control through a self-hosted relay

Status: accepted; implemented on Windows, not yet released. Task #g17 tracks qualification.

## Context

The operator wants to control a goose on one computer from another computer through a server
they host themselves (Coolify). The controlled computer must not need port forwarding. Goose
behaviour, safety rules and the local CLI/IPC contract already exist and must not be rewritten
or weakened: terminals and editors stay protected, fullscreen/DND/quiet-hours manners apply,
and ordinary stop remains the graceful walk-off.

## Decision

Add three parts, all opt-in and off by default:

1. **Relay** (`relay/`, Node 22 + `ws`, Docker). One `/ws` endpoint for geese and panels and a
   static single-page panel at `/`. Both sides authenticate in their first message with one
   shared `RELAY_TOKEN` (constant-time compare, at least 16 characters). The relay validates a
   finite command list, rate-limits panels, routes results back by relay-assigned ids, and keeps
   only in-memory state. It never invents goose behaviour.
2. **Goose client** (`src/runtime/remote.rs`). When `[remote].enabled = true`, the runtime starts
   one background thread that keeps a single outbound WebSocket (rustls + ring, bundled Mozilla
   roots) to `relay_url`, reconnects with 1–30 s backoff (5 minutes after a rejected token),
   and reports status twice a second when it changes plus a 15 s heartbeat. Every relay command
   is translated into the local IPC protocol and sent through `honk_control::send_command`,
   exactly like the CLI, so the runtime loop and engine gates stay authoritative.
3. **Panel** (`relay/public/index.html`). Goose list with live status, action buttons, and a
   mini screen map for walk-to and placing and dragging toys.

### Local IPC additions (`HONK300/1`, backward compatible)

| Frame | Response | Engine entry |
| --- | --- | --- |
| `RDO <ACTION>` | `OK` / `ERR BUSY|UNSUPPORTED|QUIET` | `World::remote_poke` |
| `WALK_TO x y` | `OK` / `ERR …` | `World::walk_to` (`WalkToTask`) |
| `TOY_PLACE B|S|C x y` | `TOY <id>` / `ERR …` | `World::place_toy` (`ToyPlayTask`) |
| `TOY_MOVE id x y` | `OK` / `ERR GONE|…` | `World::move_toy` |
| `SNAPSHOT` | `SNAP T= X= Y= L= U= W= H= Q=` | read-only |
| `TOYS` | `TOYS id:K:x,y …` (≤ 5) | read-only |

Existing frames, responses and local `DO` semantics are unchanged. All new frames fit the
128-byte frame limit; coordinates are signed desktop pixels.

### Safety

- Remote mischief respects manners: while quiet hours, DND, fullscreen or calm goose are
  active, remote requests other than `wander` (calm down) return `QUIET`. Local `DO` keeps its
  existing gates.
- Remote actions reuse existing engine actions (honk, mud, meme, note, nab, wander), so
  configuration gates, collect-window ownership and terminal protection still apply. Walk-to
  and toy placement are clamped into real monitors and never move, focus or type into any
  foreign window.
- `walk_off` maps only to graceful `STOP`; `FORCE_STOP`, reload, install, update and
  configuration are unreachable from the relay.
- Plain `ws://` is accepted only for loopback hosts. An invalid `[remote]` section disables
  remote control with a logged reason and never blocks the goose from starting.
- Remote control starts only when the Windows notification-area icon exists. While active, the
  tray tooltip, a status line in the menu and a **Disconnect remote control** item are always
  present; connecting raises a one-time notification. Disconnect ends the connection for the
  session until the `[remote]` settings change (reload) or the goose restarts.
- Toys (ball, stick, bread crumbs) are engine-owned, bounded to five, clamped into real
  monitors, and fade after three minutes. A ball rolls and bounces when the goose runs into it;
  the goose fetches a stick back to the pointer (only its own beak carries it) and eats crumbs.
  Placing a toy redirects only a wandering, playing or walking goose. Wandering geese return to
  toys occasionally, never while manners are active. Seasonal Autumn leaves are unchanged.
- Toy art is the Desktop Goose v0.31 ToysMod (ball, stick) and BreadCrumbs (crumbs) images,
  bundled as personal-use compatibility assets under `Assets/Toys/` and `relay/public/toys/`
  and listed in `THIRD_PARTY_ASSETS.md`. No mod code is used.

### Platform scope

Windows is first. macOS and Linux build the shared engine/IPC/runtime handler, but their
runtimes do not start the client until each has a menu-bar/StatusNotifier indicator and
Disconnect item with equivalent tests; a follow-up must add them before claiming parity.

## Consequences

The product gains an opt-in network client. Opening the tray menu still performs no network
check. The relay token is stored in plain text in the user's `config.toml`, like the relay's own
environment variable; anyone with it can control every goose on that relay. The relay is a
separate deployable with its own tests (`npm test` in `relay/`) and is not part of the signed
release artifacts.

## Qualification

- Engine: walk-to clamping/dwell, manners gating, toy cap/clamping/movement, ball kick physics,
  stick fetch to the pointer, eating crumbs, and frame-by-frame toy rendering.
- IPC: round-trips, malformed frames, frame-limit bounds for every new response.
- Config: default off, URL/token/name validation, untouched configs gain no `[remote]` table.
- Client: command mapping, refusal of unknown/malformed commands, Disconnect lifecycle.
- Relay: auth, routing, validation, reconnect replacement, role separation.
- Windows end to end: real relay, real runtime, panel commands, tray state and Disconnect.

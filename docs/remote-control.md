# Remote control

Control a goose on one computer from a browser anywhere, through a relay you host yourself.
Remote control is **off by default**. The decision record is
[ADR 0059](adr/0059-opt-in-remote-control-relay.md).

```
 your browser ──HTTPS/WSS──▶  relay (Coolify)  ◀──WSS (outbound)── goose on the other computer
   control panel             /      /ws                                │
                                                                        └─ local IPC (same as `goose do …`)
```

The goose opens the connection **outward**, so the controlled computer needs no port forwarding.
Every command the relay forwards is turned into the goose's existing local control commands, so
the usual safety rules still apply: terminals and editors stay protected, and fullscreen,
do-not-disturb, quiet hours and the calm-goose setting pause remote mischief.

Platform status: Windows. On macOS and Linux the `[remote]` section is accepted but ignored until
those trays gain the same indicator and Disconnect item.

## 1. Deploy the relay on Coolify

The relay is the `relay/` folder of this repository (Node 22, one dependency, Dockerfile included).

1. **Create the resource.** In Coolify: *Project → + New → Public/Private Repository*, pick your
   fork of this repo and the branch, then set:
   - **Build Pack:** `Dockerfile`
   - **Base Directory:** `/relay`
   - **Dockerfile Location:** `/Dockerfile` (relative to the base directory)
   - **Ports Exposes:** `8080`
2. **Environment variables** (*Environment Variables* tab):

   | Name | Value | Notes |
   | --- | --- | --- |
   | `RELAY_TOKEN` | a long random secret | Required, at least 16 characters. Generate one with `openssl rand -base64 32` or PowerShell `[Convert]::ToBase64String((1..32 \| % {Get-Random -Max 256}))`. Tick *Is Secret*. |
   | `PORT` | `8080` | Optional; must match *Ports Exposes*. |

   Anyone with this token can control every goose connected to the relay. Treat it like a password.
3. **Domain and HTTPS.** In *Configuration → General → Domains*, enter
   `https://goose.example.com` (use your own subdomain; point its DNS `A`/`AAAA` record at the
   Coolify server first). Coolify's proxy (Traefik or Caddy) gets a Let's Encrypt certificate
   automatically and forwards WebSocket upgrades. No extra proxy labels are needed. The goose
   connects to `wss://goose.example.com/ws`; the panel lives at `https://goose.example.com/`.
4. **Health check.** The image has a Docker `HEALTHCHECK` on `GET /healthz`. If you enable
   Coolify's own health check, use path `/healthz`, port `8080`.
5. **Deploy**, then open `https://goose.example.com/healthz`. It should answer `ok`.

Notes:
- The relay keeps everything in memory. A redeploy disconnects geese for a few seconds, and they
  reconnect by themselves.
- Run a single replica. Geese and panels must reach the same process.
- Rotate the token by changing `RELAY_TOKEN`, redeploying, and updating each goose's config.

## 2. Turn on remote control on the goose computer

Edit the goose's `config.toml` (on Windows `%LOCALAPPDATA%\honk300\config.toml`, on macOS
`~/Library/Application Support/honk300/config.toml`, on Linux `~/.local/share/honk300/config.toml`)
and add:

```toml
[remote]
enabled = true
relay_url = "wss://goose.example.com/ws"
token = "the same value as RELAY_TOKEN"
device_name = "Office PC"   # optional; defaults to the computer's name
```

Then apply it with `goose reload` (or restart the goose). Within a few seconds:

- a notification says **Goose remote control is on**;
- the tray tooltip reads *Goose controls — REMOTE CONTROL ON*;
- the tray menu starts with **● Remote control ON — goose.example.com** and
  **Disconnect remote control**.

*Disconnect remote control* closes the connection for the rest of the session. It reconnects
after you change the `[remote]` settings and reload, or after the goose restarts. To turn
remote control off permanently, set `enabled = false`.

If something is wrong (no `wss://`, token too short, bad name), the goose keeps running without
remote control and prints the reason. `ws://` is allowed only for `localhost`/`127.0.0.1` testing.

## 3. Use the control panel

Open `https://goose.example.com/`, paste the token (optionally *Remember on this device*), and
pick a goose. Each card shows whether it is in quiet mode, what it is doing and its screen size.

- **Honk, Muddy feet, Drop a meme, Drop a note, Chase cursor**: the goose's existing actions.
  Meme/note and cursor chasing follow that goose's own settings.
- **Calm down**: cancel the current errand and wander. This is the only action allowed in quiet mode.
- **Walk off (quit)**: the normal graceful walk-off. The app quits and cannot be restarted from
  the panel.
- **Screen map**: *Walk here* sends the goose to the clicked point. *Ball*, *Stick* and *Crumbs*
  drop a toy where you click (up to five, each lasts three minutes), and the goose goes straight
  for it: it kicks the ball across the screen, fetches the stick back to the mouse pointer, and
  eats the crumbs. A wandering goose also goes back to toys lying around now and then.
  *Move toy* lets you drag a toy, except while the goose is carrying it. The toy art comes from
  the Desktop Goose v0.31 ToysMod and BreadCrumbs mods (credited in `THIRD_PARTY_ASSETS.md`).

Errors are listed under *Activity*: `busy`, `turned off on this goose`, `quiet mode`,
`that toy is gone or in the goose's beak`, `goose is offline`, `no reply`.

## Protocol

### Relay WebSocket (`/ws`, JSON text frames, version 1)

Every connection must send `hello` first (within 5 s), or it is closed.

| Direction | Message |
| --- | --- |
| goose → relay | `{"type":"hello","role":"goose","v":1,"token":"…","name":"Office PC","platform":"windows","version":"1.11.3"}` |
| panel → relay | `{"type":"hello","role":"panel","v":1,"token":"…"}` |
| relay → both | `{"type":"welcome","v":1}` or `{"type":"error","error":"unauthorized"}` and close `4401` |
| goose → relay | `{"type":"status","online":true,"action":"wander","x":812,"y":604,"quiet":false,"screen":{"x":0,"y":0,"w":2560,"h":1440},"toys":[{"id":3,"kind":"ball","x":400,"y":300}]}` |
| relay → panel | `{"type":"geese","geese":[{"name":"Office PC","platform":"windows","version":"…","connectedAt":"…","status":{…}}]}` on join/leave |
| relay → panel | `{"type":"status","name":"Office PC","status":{…}}` |
| panel → relay | `{"type":"command","id":"p7","target":"Office PC","cmd":"walk_to","x":900,"y":500}` |
| relay → goose | `{"type":"command","id":"r42","cmd":"walk_to","x":900,"y":500}` |
| goose → relay | `{"type":"result","id":"r42","ok":true}`, `{"type":"result","id":"r43","ok":true,"toy":5}` or `{"type":"result","id":"r44","ok":false,"error":"QUIET"}` |
| relay → panel | `{"type":"result","id":"p7","target":"Office PC","ok":true}` |

Commands: `honk`, `mud`, `meme`, `note`, `chase_cursor`, `wander`, `walk_to {x,y}`,
`toy_place {kind,x,y}` (kind `ball`, `stick` or `crumbs`), `toy_move {toy,x,y}`, `walk_off`. Coordinates are integer desktop
pixels in the frame reported by `status.screen` (they may be negative on multi-monitor setups).

Errors from the goose: `BUSY`, `UNSUPPORTED`, `QUIET`, `GONE`, `NOT_RUNNING`, `BAD_POINT`,
`BAD_TOY`, `UNKNOWN_COMMAND`. From the relay: `offline`, `timeout` (10 s), `rate_limited`
(more than 10 commands per second), `unknown_command`, `bad_target`, `bad_point`, `bad_toy`.

Limits: 8 KiB messages, 32 geese, 16 panels, names `^[A-Za-z0-9][A-Za-z0-9 ._-]{0,39}$`. A goose
that reconnects under the same name replaces its old connection (`4409`).

### Local IPC (`HONK300/1`)

The goose client translates relay commands into these local frames (see ADR 0059):
`RDO HONK|MUD|MEME|NOTE|NAB|WANDER`, `WALK_TO x y`, `TOY_PLACE B|S|C x y`, `TOY_MOVE id x y`,
`SNAPSHOT`, `TOYS`, and the existing `STOP` for `walk_off`.

## Local testing

```bash
cd relay && npm ci && npm test
```

```bash
RELAY_TOKEN=dev-token-0123456789 PORT=8090 node relay/server.js
```

Then point a goose at `ws://127.0.0.1:8090/ws` with that token, `goose reload`, and open
`http://127.0.0.1:8090/`.

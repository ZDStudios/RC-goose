// Goose remote-control relay.
//
// Geese open an outbound WebSocket to /ws and announce themselves; the operator's browser opens
// the same endpoint as a panel. Both sides authenticate with one shared secret (RELAY_TOKEN) in
// their first message. The relay only routes the finite JSON protocol in docs/remote-control.md:
// it never invents goose behaviour, and every command is re-validated by the goose itself.
//
// Part of Goose (honk300). Licensed under PolyForm Noncommercial 1.0.0; see ../LICENSE.

"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const { WebSocketServer } = require("ws");

const PROTOCOL_VERSION = 1;
const MAX_MESSAGE_BYTES = 8 * 1024;
const AUTH_TIMEOUT_MS = 5_000;
const PING_INTERVAL_MS = 20_000;
const COMMAND_TIMEOUT_MS = 10_000;
const PANEL_COMMANDS_PER_SECOND = 10;
const MAX_GEESE = 32;
const MAX_PANELS = 16;
const NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9 ._-]{0,39}$/;

const COMMANDS = new Set([
  "honk",
  "mud",
  "meme",
  "note",
  "chase_cursor",
  "wander",
  "walk_to",
  "leaf_place",
  "leaf_move",
  "walk_off",
]);
const POINT_COMMANDS = new Set(["walk_to", "leaf_place", "leaf_move"]);

function isInt(value) {
  return Number.isSafeInteger(value) && Math.abs(value) <= 1_000_000;
}

/** Validate a panel command; returns an error string or null. */
function validateCommand(message) {
  if (typeof message.cmd !== "string" || !COMMANDS.has(message.cmd)) return "unknown_command";
  if (typeof message.target !== "string" || !NAME_PATTERN.test(message.target)) return "bad_target";
  if (POINT_COMMANDS.has(message.cmd) && !(isInt(message.x) && isInt(message.y))) return "bad_point";
  if (message.cmd === "leaf_move" && !(Number.isSafeInteger(message.leaf) && message.leaf > 0)) {
    return "bad_leaf";
  }
  return null;
}

function tokenMatches(expected, candidate) {
  if (typeof candidate !== "string") return false;
  const a = crypto.createHash("sha256").update(expected).digest();
  const b = crypto.createHash("sha256").update(candidate).digest();
  return crypto.timingSafeEqual(a, b);
}

function createRelay({ token, publicDir = path.join(__dirname, "public"), log = console.log }) {
  if (typeof token !== "string" || token.length < 16) {
    throw new Error("RELAY_TOKEN must be set to a secret of at least 16 characters");
  }

  /** name -> { socket, info, status } */
  const geese = new Map();
  const panels = new Set();
  /** relay command id -> { panel, panelId, target, timer } */
  const pending = new Map();
  let nextCommandId = 1;

  const indexHtml = fs.readFileSync(path.join(publicDir, "index.html"));

  const server = http.createServer((req, res) => {
    const url = new URL(req.url, "http://relay");
    if (req.method === "GET" && url.pathname === "/healthz") {
      res.writeHead(200, { "content-type": "text/plain" });
      res.end("ok\n");
      return;
    }
    if (req.method === "GET" && (url.pathname === "/" || url.pathname === "/index.html")) {
      res.writeHead(200, {
        "content-type": "text/html; charset=utf-8",
        "cache-control": "no-store",
        "x-content-type-options": "nosniff",
        "referrer-policy": "no-referrer",
        "x-frame-options": "DENY",
        "content-security-policy":
          "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; " +
          "connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
      });
      res.end(indexHtml);
      return;
    }
    res.writeHead(404, { "content-type": "text/plain" });
    res.end("not found\n");
  });

  const wss = new WebSocketServer({ server, path: "/ws", maxPayload: MAX_MESSAGE_BYTES });

  function send(socket, message) {
    if (socket.readyState === socket.OPEN) socket.send(JSON.stringify(message));
  }

  function geeseSummary() {
    return [...geese.entries()].map(([name, goose]) => ({
      name,
      platform: goose.info.platform,
      version: goose.info.version,
      connectedAt: goose.info.connectedAt,
      status: goose.status,
    }));
  }

  function broadcastGeese() {
    const message = { type: "geese", geese: geeseSummary() };
    for (const panel of panels) send(panel, message);
  }

  function finishCommand(relayId, result) {
    const entry = pending.get(relayId);
    if (!entry) return;
    pending.delete(relayId);
    clearTimeout(entry.timer);
    send(entry.panel, { type: "result", id: entry.panelId, target: entry.target, ...result });
  }

  function onGooseMessage(socket, name, message) {
    const goose = geese.get(name);
    if (!goose || goose.socket !== socket) return;
    if (message.type === "status") {
      goose.status = sanitizeStatus(message);
      const update = { type: "status", name, status: goose.status };
      for (const panel of panels) send(panel, update);
    } else if (message.type === "result" && typeof message.id === "string") {
      const result = { ok: message.ok === true };
      if (typeof message.error === "string") result.error = message.error.slice(0, 32);
      if (Number.isSafeInteger(message.leaf)) result.leaf = message.leaf;
      finishCommand(message.id, result);
    }
  }

  function onPanelMessage(socket, message) {
    if (message.type !== "command") return;
    const now = Date.now();
    socket.rate = socket.rate.filter((at) => now - at < 1_000);
    const panelId = typeof message.id === "string" ? message.id.slice(0, 64) : null;
    const reply = (error) =>
      send(socket, { type: "result", id: panelId, target: message.target, ok: false, error });
    if (socket.rate.length >= PANEL_COMMANDS_PER_SECOND) return reply("rate_limited");
    socket.rate.push(now);
    const error = validateCommand(message);
    if (error) return reply(error);
    const goose = geese.get(message.target);
    if (!goose) return reply("offline");

    const relayId = `r${nextCommandId++}`;
    const command = { type: "command", id: relayId, cmd: message.cmd };
    if (POINT_COMMANDS.has(message.cmd)) {
      command.x = message.x;
      command.y = message.y;
    }
    if (message.cmd === "leaf_move") command.leaf = message.leaf;
    const timer = setTimeout(() => finishCommand(relayId, { ok: false, error: "timeout" }), COMMAND_TIMEOUT_MS);
    pending.set(relayId, { panel: socket, panelId, target: message.target, timer });
    send(goose.socket, command);
    log(`command ${message.cmd} -> ${message.target}`);
  }

  function authenticate(socket, message) {
    if (message.type !== "hello" || !tokenMatches(token, message.token)) {
      send(socket, { type: "error", error: "unauthorized" });
      socket.close(4401, "unauthorized");
      return;
    }
    if (message.v !== PROTOCOL_VERSION) {
      send(socket, { type: "error", error: "unsupported_version" });
      socket.close(4400, "unsupported version");
      return;
    }
    if (message.role === "goose") {
      if (typeof message.name !== "string" || !NAME_PATTERN.test(message.name)) {
        send(socket, { type: "error", error: "bad_name" });
        socket.close(4400, "bad name");
        return;
      }
      const name = message.name;
      const previous = geese.get(name);
      if (!previous && geese.size >= MAX_GEESE) {
        send(socket, { type: "error", error: "too_many_geese" });
        socket.close(4429, "too many geese");
        return;
      }
      if (previous) {
        // A reconnect after a dropped network replaces the stale socket under the same name.
        previous.socket.close(4409, "replaced by a newer connection");
      }
      geese.set(name, {
        socket,
        info: {
          platform: typeof message.platform === "string" ? message.platform.slice(0, 16) : "unknown",
          version: typeof message.version === "string" ? message.version.slice(0, 32) : "unknown",
          connectedAt: new Date().toISOString(),
        },
        status: null,
      });
      socket.role = "goose";
      socket.gooseName = name;
      send(socket, { type: "welcome", v: PROTOCOL_VERSION });
      log(`goose connected: ${name}`);
      broadcastGeese();
    } else if (message.role === "panel") {
      if (panels.size >= MAX_PANELS) {
        send(socket, { type: "error", error: "too_many_panels" });
        socket.close(4429, "too many panels");
        return;
      }
      socket.role = "panel";
      socket.rate = [];
      panels.add(socket);
      send(socket, { type: "welcome", v: PROTOCOL_VERSION });
      send(socket, { type: "geese", geese: geeseSummary() });
      log("panel connected");
    } else {
      send(socket, { type: "error", error: "bad_role" });
      socket.close(4400, "bad role");
    }
  }

  wss.on("connection", (socket) => {
    socket.isAlive = true;
    socket.on("pong", () => {
      socket.isAlive = true;
    });
    const authTimer = setTimeout(() => {
      if (!socket.role) socket.close(4401, "authentication timeout");
    }, AUTH_TIMEOUT_MS);

    socket.on("message", (data, isBinary) => {
      let message;
      try {
        if (isBinary) throw new Error("binary");
        message = JSON.parse(data.toString("utf8"));
        if (!message || typeof message !== "object" || Array.isArray(message)) throw new Error("shape");
      } catch {
        socket.close(4400, "malformed message");
        return;
      }
      if (!socket.role) {
        clearTimeout(authTimer);
        authenticate(socket, message);
      } else if (socket.role === "goose") {
        onGooseMessage(socket, socket.gooseName, message);
      } else {
        onPanelMessage(socket, message);
      }
    });

    socket.on("close", () => {
      clearTimeout(authTimer);
      if (socket.role === "goose") {
        const goose = geese.get(socket.gooseName);
        if (goose && goose.socket === socket) {
          geese.delete(socket.gooseName);
          log(`goose disconnected: ${socket.gooseName}`);
          for (const [relayId, entry] of pending) {
            if (entry.target === socket.gooseName) finishCommand(relayId, { ok: false, error: "offline" });
          }
          broadcastGeese();
        }
      } else if (socket.role === "panel") {
        panels.delete(socket);
        for (const [relayId, entry] of pending) {
          if (entry.panel === socket) {
            clearTimeout(entry.timer);
            pending.delete(relayId);
          }
        }
      }
    });
    socket.on("error", () => {});
  });

  const pinger = setInterval(() => {
    for (const socket of wss.clients) {
      if (!socket.isAlive) {
        socket.terminate();
        continue;
      }
      socket.isAlive = false;
      socket.ping();
    }
  }, PING_INTERVAL_MS);

  return {
    server,
    listen(port, host) {
      return new Promise((resolve) => server.listen(port, host, () => resolve(server.address())));
    },
    close() {
      clearInterval(pinger);
      for (const socket of wss.clients) socket.terminate();
      return new Promise((resolve) => wss.close(() => server.close(() => resolve())));
    },
  };
}

function sanitizeStatus(message) {
  const int = (value) => (isInt(value) ? value : null);
  const screen = message.screen && typeof message.screen === "object" ? message.screen : {};
  const leaves = Array.isArray(message.leaves) ? message.leaves.slice(0, 16) : [];
  return {
    online: message.online === true,
    action: typeof message.action === "string" ? message.action.slice(0, 32) : "unknown",
    quiet: message.quiet === true,
    x: int(message.x),
    y: int(message.y),
    screen: { x: int(screen.x), y: int(screen.y), w: int(screen.w), h: int(screen.h) },
    leaves: leaves
      .filter((leaf) => leaf && Number.isSafeInteger(leaf.id) && isInt(leaf.x) && isInt(leaf.y))
      .map((leaf) => ({ id: leaf.id, x: leaf.x, y: leaf.y })),
  };
}

module.exports = { createRelay, validateCommand, PROTOCOL_VERSION };

if (require.main === module) {
  const port = Number.parseInt(process.env.PORT || "8080", 10);
  const relay = createRelay({ token: process.env.RELAY_TOKEN });
  relay.listen(port, "0.0.0.0").then(() => console.log(`goose relay listening on :${port}`));
  const shutdown = () => relay.close().then(() => process.exit(0));
  process.on("SIGTERM", shutdown);
  process.on("SIGINT", shutdown);
}

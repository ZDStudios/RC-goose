"use strict";

const assert = require("node:assert/strict");
const { test, beforeEach, afterEach } = require("node:test");
const WebSocket = require("ws");
const { createRelay, validateCommand } = require("../server");

const TOKEN = "test-token-0123456789";
let relay;
let base;

beforeEach(async () => {
  relay = createRelay({ token: TOKEN, log: () => {} });
  const address = await relay.listen(0, "127.0.0.1");
  base = `ws://127.0.0.1:${address.port}/ws`;
});

afterEach(async () => {
  await relay.close();
});

/** Open a socket and collect its messages so tests can await specific ones. */
function connect() {
  const socket = new WebSocket(base);
  const inbox = [];
  const waiters = [];
  socket.on("message", (data) => {
    const message = JSON.parse(data.toString());
    const index = waiters.findIndex((w) => w.match(message));
    if (index >= 0) waiters.splice(index, 1)[0].resolve(message);
    else inbox.push(message);
  });
  const closed = new Promise((resolve) => socket.on("close", (code) => resolve(code)));
  return {
    socket,
    closed,
    opened: new Promise((resolve) => socket.on("open", resolve)),
    send: (message) => socket.send(JSON.stringify(message)),
    next(match = () => true) {
      const index = inbox.findIndex(match);
      if (index >= 0) return Promise.resolve(inbox.splice(index, 1)[0]);
      return new Promise((resolve) => waiters.push({ match, resolve }));
    },
  };
}

async function goose(name) {
  const client = connect();
  await client.opened;
  client.send({ type: "hello", role: "goose", v: 1, token: TOKEN, name, platform: "test", version: "0" });
  assert.equal((await client.next()).type, "welcome");
  return client;
}

async function panel() {
  const client = connect();
  await client.opened;
  client.send({ type: "hello", role: "panel", v: 1, token: TOKEN });
  assert.equal((await client.next()).type, "welcome");
  return client;
}

test("rejects a wrong token and closes", async () => {
  const client = connect();
  await client.opened;
  client.send({ type: "hello", role: "panel", v: 1, token: "nope" });
  assert.deepEqual(await client.next(), { type: "error", error: "unauthorized" });
  assert.equal(await client.closed, 4401);
});

test("refuses to start without a strong token", () => {
  assert.throws(() => createRelay({ token: "short" }));
  assert.throws(() => createRelay({ token: undefined }));
});

test("panel sees geese join, report status, and leave", async () => {
  const p = await panel();
  assert.deepEqual((await p.next((m) => m.type === "geese")).geese, []);
  const g = await goose("office-pc");
  const joined = await p.next((m) => m.type === "geese");
  assert.equal(joined.geese[0].name, "office-pc");

  g.send({ type: "status", online: true, action: "wander", x: 5, y: 6, quiet: false,
    screen: { x: 0, y: 0, w: 1920, h: 1080 }, leaves: [{ id: 2, x: 1, y: 2 }, { id: "bad" }] });
  const status = await p.next((m) => m.type === "status");
  assert.equal(status.name, "office-pc");
  assert.equal(status.status.action, "wander");
  assert.deepEqual(status.status.leaves, [{ id: 2, x: 1, y: 2 }]);

  g.socket.close();
  const left = await p.next((m) => m.type === "geese" && m.geese.length === 0);
  assert.deepEqual(left.geese, []);
});

test("routes a command to the named goose and the result back to the panel", async () => {
  const p = await panel();
  const g = await goose("laptop");
  p.send({ type: "command", id: "p1", target: "laptop", cmd: "walk_to", x: 100, y: -20 });
  const command = await g.next((m) => m.type === "command");
  assert.equal(command.cmd, "walk_to");
  assert.equal(command.x, 100);
  assert.equal(command.y, -20);
  assert.equal(command.target, undefined, "the goose never sees routing metadata");

  g.send({ type: "result", id: command.id, ok: true });
  assert.deepEqual(await p.next((m) => m.type === "result"), {
    type: "result", id: "p1", target: "laptop", ok: true,
  });
});

test("reports offline targets and invalid commands without forwarding", async () => {
  const p = await panel();
  p.send({ type: "command", id: "a", target: "ghost", cmd: "honk" });
  assert.equal((await p.next((m) => m.id === "a")).error, "offline");
  p.send({ type: "command", id: "b", target: "ghost", cmd: "rm -rf" });
  assert.equal((await p.next((m) => m.id === "b")).error, "unknown_command");
  p.send({ type: "command", id: "c", target: "ghost", cmd: "walk_to", x: 1.5, y: 2 });
  assert.equal((await p.next((m) => m.id === "c")).error, "bad_point");
});

test("a reconnecting goose replaces its stale socket", async () => {
  const first = await goose("desk");
  const second = await goose("desk");
  assert.equal(await first.closed, 4409);
  const p = await panel();
  const list = await p.next((m) => m.type === "geese");
  assert.equal(list.geese.length, 1);
  p.send({ type: "command", id: "x", target: "desk", cmd: "honk" });
  assert.equal((await second.next((m) => m.type === "command")).cmd, "honk");
});

test("geese cannot send commands and panels cannot fake status", async () => {
  const p = await panel();
  const g = await goose("victim");
  await p.next((m) => m.type === "geese" && m.geese.length === 1);
  g.send({ type: "command", id: "evil", target: "victim", cmd: "honk" });
  p.send({ type: "status", name: "victim", online: false });
  p.send({ type: "command", id: "ok", target: "victim", cmd: "honk" });
  const command = await g.next((m) => m.type === "command");
  assert.equal(command.cmd, "honk");
});

test("validateCommand covers every point command", () => {
  assert.equal(validateCommand({ target: "a", cmd: "leaf_move", x: 1, y: 2 }), "bad_leaf");
  assert.equal(validateCommand({ target: "a", cmd: "leaf_move", x: 1, y: 2, leaf: 3 }), null);
  assert.equal(validateCommand({ target: "a b", cmd: "leaf_place", x: 1, y: 2 }), null);
  assert.equal(validateCommand({ target: "", cmd: "honk" }), "bad_target");
});

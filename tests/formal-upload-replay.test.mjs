import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// Finite ABSTRACT reconciliation model, not a proof about the Opto-Sync runtime.
// A future runtime adapter must replay the same transitions against real storage.
const schema = JSON.parse(readFileSync(new URL("../schema/upload-job.schema.json", import.meta.url)));
const SIZE = 3;
const MAX_ATTEMPT = 2;
const terminal = new Set(["imported", "cancelled"]);
const statuses = new Set(schema.properties.status.enum);

function state(status = "queued", checkpoint = 0, attempt = 0) {
  return { status, checkpoint, attempt };
}
function assertState(s) {
  if (!statuses.has(s.status) ||
      !Number.isSafeInteger(s.checkpoint) || s.checkpoint < 0 || s.checkpoint > SIZE ||
      !Number.isSafeInteger(s.attempt) || s.attempt < 0 || s.attempt > MAX_ATTEMPT)
    throw new RangeError("invalid bounded upload state");
}
function reconcileCheckpoint(s, remote) {
  assertState(s);
  if (!Number.isSafeInteger(remote) || remote < 0 || remote > SIZE)
    throw new RangeError("invalid remote checkpoint");
  return state(s.status, Math.max(s.checkpoint, remote), s.attempt);
}
function successors(s) {
  assertState(s);
  if (terminal.has(s.status)) return [];
  const out = [];
  const put = (status = s.status, checkpoint = s.checkpoint, attempt = s.attempt) =>
    out.push(state(status, checkpoint, attempt));
  if (s.status === "queued") put("declaring");
  if (s.status === "declaring") {
    put("uploading");
    put("failed");
  }
  if (s.status === "uploading" && s.checkpoint < SIZE)
    put("uploading", s.checkpoint + 1);
  if (s.status === "uploading") {
    put("paused");
    put("failed");
  }
  if (s.status === "failed" && s.attempt < MAX_ATTEMPT)
    put("queued", s.checkpoint, s.attempt + 1);
  if (s.status === "paused" && s.attempt < MAX_ATTEMPT)
    put("uploading", s.checkpoint, s.attempt + 1);
  if (s.status === "uploading" && s.checkpoint === SIZE) put("available");
  if (s.status === "available") put("imported");
  put("cancelled");
  // A delayed server snapshot never rolls back unconfirmed local progress.
  for (let remote = 0; remote <= SIZE; remote++)
    out.push(reconcileCheckpoint(s, remote));
  return out;
}

test("replication contract bounds checkpoints and excludes local capabilities", () => {
  assert.deepEqual(schema.properties.status.enum,
    ["queued", "declaring", "uploading", "paused", "available", "imported", "failed", "cancelled"]);
  for (const field of ["local_ref", "capability", "pairing_secret", "event_ticket", "presigned_url", "bytes"])
    assert.equal(Object.hasOwn(schema.properties, field), false, field);
  assert.equal(schema.additionalProperties, false);
});

test("every reachable abstract replay preserves progress, retry bound and terminality", () => {
  const first = state();
  const seen = new Set();
  const queue = [[first, []]];
  let transitions = 0;
  let sawRetry = false, sawImport = false, sawCancel = false;
  let sawFailure = false, sawRecovery = false, sawStaleSnapshot = false;
  while (queue.length) {
    const [current, trace] = queue.shift();
    const id = JSON.stringify(current);
    if (seen.has(id)) continue;
    seen.add(id);
    assert.ok(current.checkpoint >= 0 && current.checkpoint <= SIZE, trace.join(" -> "));
    assert.ok(current.attempt >= 0 && current.attempt <= MAX_ATTEMPT, trace.join(" -> "));
    if (current.status === "imported") sawImport = true;
    if (current.status === "cancelled") sawCancel = true;
    if (current.status === "failed") sawFailure = true;
    if (current.checkpoint > 0)
      for (let remote = 0; remote < current.checkpoint; remote++) {
        const next = reconcileCheckpoint(current, remote);
        assert.equal(next.checkpoint, current.checkpoint);
        sawStaleSnapshot = true; // witness requires remote < local, not arbitrary stutter
      }
    const children = successors(current);
    if (terminal.has(current.status)) assert.equal(children.length, 0);
    for (const next of children) {
      const path = [...trace, id];
      transitions++;
      assert.ok(next.checkpoint >= current.checkpoint, path.join(" -> "));
      assert.ok(next.attempt >= current.attempt, path.join(" -> "));
      if (next.attempt > current.attempt) sawRetry = true;
      if (current.status === "failed" && next.status === "queued") sawRecovery = true;
      queue.push([next, path]);
    }
  }
  assert.ok(seen.size >= 20 && transitions >= 40, "nonvacuous bounded exploration");
  assert.ok(sawRetry && sawImport && sawCancel && sawStaleSnapshot);
  assert.ok(sawFailure && sawRecovery, "failure and recovery witnesses required");
});

test("stale server snapshots preserve every local checkpoint, including after failure", () => {
  for (const status of ["queued", "declaring", "uploading", "paused", "failed", "available"])
    for (let local = 1; local <= SIZE; local++)
      for (let remote = 0; remote < local; remote++) {
        const initial = state(status, local, 1);
        assert.deepEqual(reconcileCheckpoint(initial, remote), initial);
      }
});

test("invalid and noninteger remote checkpoints fail closed instead of being clamped", () => {
  for (const invalid of [-1, SIZE + 1, Number.NaN, Infinity, 0.5, "2", null])
    assert.throws(() => reconcileCheckpoint(state(), invalid), RangeError);
});

test("malformed local state and over-budget retries are rejected", () => {
  for (const invalid of [
    state("unexpected", 0, 0), state("queued", -1, 0),
    state("queued", SIZE + 1, 0), state("queued", 0, MAX_ATTEMPT + 1),
    state("queued", 0.25, 0), state("queued", 0, -1),
  ]) assert.throws(() => successors(invalid), RangeError);
});

test("terminal states reject all updates and retries, not just normal transitions", () => {
  for (const status of terminal) {
    const terminalState = state(status, SIZE, MAX_ATTEMPT);
    assert.deepEqual(successors(terminalState), []);
    // A terminal job cannot be resurrected by a stale remote snapshot.
    assert.deepEqual(successors(terminalState), []);
  }
});

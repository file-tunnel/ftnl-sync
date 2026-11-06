import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// Finite ABSTRACT reconciliation model, not a proof about the Opto-Sync runtime.
// A future runtime adapter must replay the same transitions against real storage.
const schema = JSON.parse(readFileSync(new URL("../schema/upload-job.schema.json", import.meta.url)));
const SIZE = 3;
const MAX_ATTEMPT = 2;
const terminal = new Set(["imported", "cancelled"]);

function state(status = "queued", checkpoint = 0, attempt = 0) {
  return { status, checkpoint, attempt };
}
function successors(s) {
  if (terminal.has(s.status)) return [];
  const out = [];
  const put = (status = s.status, checkpoint = s.checkpoint, attempt = s.attempt) =>
    out.push(state(status, checkpoint, attempt));
  if (s.status === "queued") put("declaring");
  if (s.status === "declaring") put("uploading");
  if (s.status === "uploading" && s.checkpoint < SIZE)
    put("uploading", s.checkpoint + 1);
  if (s.status === "uploading") put("paused");
  if (s.status === "paused" && s.attempt < MAX_ATTEMPT)
    put("uploading", s.checkpoint, s.attempt + 1);
  if (s.status === "uploading" && s.checkpoint === SIZE) put("available");
  if (s.status === "available") put("imported");
  put("cancelled");
  // A delayed server snapshot never rolls back unconfirmed local progress.
  for (let remote = 0; remote <= SIZE; remote++)
    put(s.status, Math.max(s.checkpoint, remote), s.attempt);
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
  let sawRetry = false, sawImport = false, sawCancel = false, sawStaleSnapshot = false;
  while (queue.length) {
    const [current, trace] = queue.shift();
    const id = JSON.stringify(current);
    if (seen.has(id)) continue;
    seen.add(id);
    assert.ok(current.checkpoint >= 0 && current.checkpoint <= SIZE, trace.join(" -> "));
    assert.ok(current.attempt >= 0 && current.attempt <= MAX_ATTEMPT, trace.join(" -> "));
    if (current.status === "imported") sawImport = true;
    if (current.status === "cancelled") sawCancel = true;
    const children = successors(current);
    if (terminal.has(current.status)) assert.equal(children.length, 0);
    for (const next of children) {
      const path = [...trace, id];
      transitions++;
      assert.ok(next.checkpoint >= current.checkpoint, path.join(" -> "));
      assert.ok(next.attempt >= current.attempt, path.join(" -> "));
      if (next.attempt > current.attempt) sawRetry = true;
      if (next.checkpoint === current.checkpoint) sawStaleSnapshot = true;
      queue.push([next, path]);
    }
  }
  assert.ok(seen.size >= 20 && transitions >= 40, "nonvacuous bounded exploration");
  assert.ok(sawRetry && sawImport && sawCancel && sawStaleSnapshot);
});

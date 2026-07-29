import assert from "node:assert/strict";
import { test } from "node:test";
import { FileTunnelSync } from "../dist/index.js";

const job = {
  id: "018f47d2-2d9f-7a41-a2aa-1aef7d847001",
  tunnel_id: "018f47d2-2d9f-7a41-a2aa-1aef7d847002",
  file_id: null,
  name: "photo.jpg",
  media_type: "image/jpeg",
  size_bytes: 100,
  bytes_transferred: 0,
  status: "queued",
  attempt: 0,
  reason_code: null,
  updatedAt: "1722276000000-0-device",
  syncedAt: null,
};

test("only replication-safe metadata enters opto-sync", async () => {
  const mutations = [];
  const locals = new Map();
  const sync = new FileTunnelSync(
    { queueMutation: async (...args) => mutations.push(args) },
    {
      put: async (record, localRef) => locals.set(record.id, localRef),
      getLocalRef: async (id) => locals.get(id),
    },
  );
  const localRef = { sensitiveDeviceHandle: true };
  await sync.enqueue(job, localRef);
  assert.equal(mutations.length, 1);
  const payload = JSON.parse(mutations[0][2]);
  assert.equal(payload.name, "photo.jpg");
  assert.equal(JSON.stringify(payload).includes("sensitiveDeviceHandle"), false);
});

test("raw error details cannot enter replicated state", async () => {
  const sync = new FileTunnelSync(
    { queueMutation: async () => {} },
    { put: async () => {}, getLocalRef: async () => undefined },
  );
  await assert.rejects(
    sync.transition(job, { reason_code: "Bearer secret-token leaked by proxy" }),
    /allowlisted/,
  );
});

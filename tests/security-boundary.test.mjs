import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

test("replication schema excludes secrets, local handles, and content", async () => {
  const schema = JSON.parse(
    await readFile(new URL("../schema/upload-job.schema.json", import.meta.url), "utf8"),
  );
  const fields = Object.keys(schema.properties);
  for (const forbidden of [
    "capability",
    "desktop_capability",
    "phone_capability",
    "pairing_secret",
    "event_ticket",
    "presigned_url",
    "local_ref",
    "content",
    "bytes",
  ]) {
    assert.ok(!fields.includes(forbidden), `${forbidden} must stay local-only`);
  }
  assert.equal(schema.additionalProperties, false);
});

test("the local schema identifies its non-replicated reference", async () => {
  const sql = await readFile(
    new URL("../sql/001_local_upload_jobs.sql", import.meta.url),
    "utf8",
  );
  assert.match(sql, /local_ref TEXT NOT NULL/);
  assert.doesNotMatch(sql, /capability|pairing_secret|event_ticket/i);
});

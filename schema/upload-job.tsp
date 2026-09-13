// File Tunnel replication-safe upload job — TypeSpec peer authority.
//
// This file and schema/upload-job.schema.json are independent, human-authored
// peers for the record opto-sync replicates between File Tunnel devices and the
// backend (adapters/rust, adapters/typescript, sql/001_local_upload_jobs.sql).
// Neither is generated from the other. TJSV
// (ORESoftware/typespec-json-schema-validator) emits a comparison-only JSON
// Schema from this file and fails closed unless it agrees with the authored
// schema. Local-only handles (local_ref, capabilities, pairing secrets, presigned
// URLs, content bytes) are deliberately absent from both.
import "@typespec/json-schema";
using TypeSpec.JsonSchema;

namespace FileTunnel.Sync;

@doc("Replicated upload job record. Every field is required; absent values are explicit nulls.")
model UploadJob {
  @format("uuid") id: string;
  @format("uuid") tunnel_id: string;

  @doc("Null until the backend has declared the file.")
  @format("uuid")
  file_id: string | null;

  @minLength(1) @maxLength(255) name: string;
  @minLength(1) @maxLength(128) media_type: string;

  @doc("Declared size; the File Tunnel ceiling is 5 GiB.")
  @minValue(0)
  @maxValue(5368709120)
  size_bytes: integer;

  @minValue(0) bytes_transferred: integer;

  @doc("queued → declaring → uploading ⇄ paused → available → imported, plus failed and cancelled.")
  status: "queued" | "declaring" | "uploading" | "paused" | "available" | "imported" | "failed" | "cancelled";

  @minValue(0) @maxValue(100) attempt: integer;

  @maxLength(64) reason_code: string | null;

  @doc("opto-sync HLC stamp: 13-digit Unix millis, 4-hex counter, node id.")
  @pattern("^[0-9]{13}-[0-9a-fA-F]{4}-[A-Za-z0-9._:]+$")
  updatedAt: string;

  syncedAt: string | null;
}

export type UploadStatus =
  | "queued"
  | "declaring"
  | "uploading"
  | "paused"
  | "available"
  | "imported"
  | "failed"
  | "cancelled";

export interface UploadJob {
  id: string;
  tunnel_id: string;
  file_id: string | null;
  name: string;
  media_type: string;
  size_bytes: number;
  bytes_transferred: number;
  status: UploadStatus;
  attempt: number;
  reason_code: string | null;
  updatedAt: string;
  syncedAt: string | null;
}

/**
 * Structural seam implemented by opto-sync's IndexedDB client.
 *
 * Keep this structural rather than importing its class: applications may
 * inject the browser/WASM or Node/native entry point without this package
 * accidentally selecting the wrong runtime at bundle time.
 */
export interface OptoMutationQueue {
  queueMutation(
    tableName: string,
    recordId: string,
    jsonPayload: string,
  ): Promise<unknown>;
}

export interface LocalUploadStore {
  put(job: UploadJob, localRef: unknown): Promise<void>;
  getLocalRef(jobId: string): Promise<unknown | undefined>;
}

const allowedReasons = new Set([
  "network_unavailable",
  "permission_required",
  "source_missing",
  "tunnel_expired",
  "file_rejected",
  "upload_interrupted",
]);
const canonicalHlc = /^[0-9]{13}-[0-9a-fA-F]{4}-[A-Za-z0-9._:]+$/;

export class FileTunnelSync {
  constructor(
    private readonly opto: OptoMutationQueue,
    private readonly local: LocalUploadStore,
  ) {}

  async enqueue(job: UploadJob, localRef: unknown): Promise<void> {
    validate(job);
    await this.local.put(job, localRef);
    await this.queue(job);
  }

  async transition(
    current: UploadJob,
    patch: Partial<Pick<UploadJob, "file_id" | "bytes_transferred" | "status" | "attempt" | "reason_code" | "updatedAt" | "syncedAt">>,
  ): Promise<UploadJob> {
    const next = { ...current, ...patch };
    validate(next);
    await this.queue(next);
    return next;
  }

  private async queue(job: UploadJob): Promise<void> {
    // JSON serialization is the security boundary. UploadJob has no localRef,
    // content, bearer token, or provider URL field.
    await this.opto.queueMutation("ftnl_upload_jobs", job.id, JSON.stringify(job));
  }
}

function validate(job: UploadJob): void {
  if (!job.id || !job.tunnel_id || !job.name || !job.media_type) {
    throw new TypeError("upload job identity and display metadata are required");
  }
  if (job.name.length > 255 || job.media_type.length > 128) {
    throw new RangeError("upload job display metadata exceeds the replication contract");
  }
  if (!Number.isSafeInteger(job.size_bytes) || job.size_bytes < 0 || job.size_bytes > 5_368_709_120) {
    throw new RangeError("size_bytes must be within the replication contract");
  }
  if (
    !Number.isSafeInteger(job.bytes_transferred) ||
    job.bytes_transferred < 0 ||
    job.bytes_transferred > job.size_bytes
  ) {
    throw new RangeError("bytes_transferred must be within the declared size");
  }
  if (job.reason_code && !allowedReasons.has(job.reason_code)) {
    throw new TypeError("reason_code must be redacted and allowlisted");
  }
  if (!Number.isSafeInteger(job.attempt) || job.attempt < 0 || job.attempt > 100) {
    throw new RangeError("attempt must be an integer from 0 through 100");
  }
  if (!canonicalHlc.test(job.updatedAt)) {
    throw new TypeError("updatedAt must be a canonical opto-sync HLC timestamp");
  }
}

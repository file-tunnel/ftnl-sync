# Security policy

Report vulnerabilities with GitHub Security Advisories.

The replication-safe model must never grow fields for content bytes, local
paths/handles, bearer capabilities, pairing secrets, event tickets, presigned
URLs, or provider-specific object keys. CI treats those field names as a
contract violation. Error details must be reduced to an allowlisted reason
code before entering the queue.

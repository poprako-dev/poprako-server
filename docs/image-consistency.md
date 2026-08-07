# Image Upload Consistency Contract

This document is the authoritative safety contract for page images, user
avatars, team avatars, and comic covers. The four image owners use the same
reservation identity, verification rules, and delayed correction semantics.

## Identity and reservation

An image reservation request contains:

- `image_hash`: the SHA-256 digest of the exact object bytes;
- `new_byte_len`: the expected object size, from 1 MiB through 20 MiB;
- `ext`: the normalized file extension.

The persisted upload identity is the complete tuple `(owner id, image_version,
key, image_hash, ext)`. `new_byte_len` is not part of that identity. It is used
only to validate a reserve request and constrain a presigned PUT. It is neither
persisted nor returned and is never compared with object metadata after upload.

Every PUT capability is represented by one shared upload slot:

```text
put_url + image_version + headers
```

The headers bind the PUT to both the SHA-256 checksum and content length.
Single-owner reserve responses contain `slot: Option<UploadSlotVal>`. Page
batch responses retain one optional slot per page.

For an existing image identity:

- A matching, uploaded identity returns no slot and changes no state or task.
- A matching, pending identity keeps its key and version, issues a new PUT
  capability, and replaces the delayed checks for that identity.
- A changed hash or extension increments the version, creates a versioned key, sets
  `uploaded` to false, and atomically creates a dedicated Delete task for the
  previous key.

An owner without an image persists every image field as `NULL`: key, uploaded,
version, hash, and extension. An existing image persists that whole tuple; a
partially populated tuple is unrecoverable persisted data. User, team, and
comic begin in the all-`NULL` state, and their first reservation uses version
1. Page creation is different: each new page must contain a real image
identity and begins as explicitly pending (`uploaded = false`). A page does
not persist `new_byte_len`.

Chapter manifest entries may omit `new_byte_len` only when they retain an
existing page without requesting an upload slot. A pending page with
`new_byte_len` reuses its key and version and receives a fresh slot. New pages
and changed identities require `new_byte_len`.

## Lock order and transaction boundary

Every operation that can observe or mutate a page upload follows one lock
order:

```text
Chapter FOR UPDATE -> Page FOR UPDATE
```

This includes single and batch reserve, mark accounting, CheckUpload
correction, and AdvanceRawProvide. The common order prevents both deadlocks
and chapter-summary races. Code implementing these paths carries an English
`NOTE:` comment documenting the invariant.

Object storage HEAD calls never run while a database transaction is open.
After HEAD succeeds, the accounting transaction locks the aggregate again and
compares the complete persisted identity `(id, version, key, hash, ext)`.
Only an exact match may be changed.

## Mark-uploaded

The mark request contains only `image_version`.

1. Authenticate and read the current persisted identity.
2. If the requested current identity is already uploaded, return success
   without calling object storage.
3. Otherwise HEAD the current key outside the transaction.
4. Require a present SHA-256 checksum equal to the reserved hash.
5. Lock the aggregate, re-check the complete identity, and perform an exact
   false-to-true compare-and-set.

A missing object or mismatched hash is an argument error. A cleared or stale
identity is handled as stale and cannot recreate a key, hash, extension, or
version.
Object-storage failures and missing checksum metadata are internal errors.
Every failure leaves persisted state unchanged. Mark never advances or rolls
back RawProvide and never deletes an unexpected object.

## Delayed tasks

### CheckUpload (15 minutes)

Every pending reservation creates a CheckUpload task for its exact identity.
The handler performs HEAD outside the accounting transaction.

- A current object with the correct hash is precisely set to uploaded.
- A current object that is missing or has the wrong hash is precisely set to
  not uploaded.
- A wrong-hash object for the current identity is deleted by this task after
  the PUT URL has expired.
- A stale, cleared, or deleted owner completes without deleting the payload key.
  Cleanup of stale keys belongs exclusively to dedicated Delete tasks.
- The same version with a different key is an internal invariant violation and
  makes the task Dead. The handler does not guess which key is safe to delete.

Implementations carry an English `SAFETY:` comment at the stale/deleted
boundary. CheckUpload never completes or resets RawProvide.

### AdvanceRawProvide (20 minutes)

Every page batch reservation creates exactly one AdvanceRawProvide task,
including a batch that creates no new upload slot. A single-page replacement
that creates an upload slot also creates one.

AdvanceRawProvide is the only automatic RawProvide transition. In one
transaction it locks the chapter, observes all pages, and:

- changes RawProvide from Pending to Completed when every page explicitly has
  `uploaded = true`;
- otherwise completes the task as a logical negative result without retrying.

Only infrastructure failures retry, at five-minute intervals, at most three
times before Dead. Only a real Pending-to-Completed transition emits the same
`ChapterWorkflowCompleted(RawProvide)` effect as manual completion. It never
reverts a manually completed stage. Once a task is Dead, manual progression
remains available.

Successful verification followed by later external object loss is outside this
one-shot upload protocol.

## Publishing pages

Publishing collects the current page object keys for deletion and then clears
each page's entire image tuple in the same transaction. Published pages retain
no synthetic zero hash, default extension, or version zero. Read APIs omit the
image URL, thumbnail URL, hash, and extension in this state. Any later upload
or check task for the former tuple is stale and must not resurrect it.

## Prom claim fencing

A worker attempt is identified by `(message_id, lease)`.

- Claim matches Pending and the expected current lease, changes the row to
  Processing, and returns the new attempt lease.
- Complete, Retry, and Fail match both Processing and the attempt lease.
- Zero affected rows means that the attempt expired; its finalization is
  silently ignored and cannot overwrite a newer lease or Dead state.
- Stuck reset increments the existing lease before making work available
  again.

Handlers must remain idempotent and use identity compare-and-set because lease
fencing protects queue state, not arbitrary business writes.

## Runtime ownership and shutdown

Prom image/chapter handlers share the effect developer used by manual workflow
completion. Production composition passes a shared reference. Shutdown first
drains and closes Prom, then closes the effect developer, so a final automatic
workflow event is not dropped.

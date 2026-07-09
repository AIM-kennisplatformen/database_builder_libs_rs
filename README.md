# Database Builder SCEPA

Builds structured paper data from PDFs. The current pipeline extracts TEI XML with
GROBID, parses it into internal models, transforms it into the domain `Paper`
model, and exports it to JSON, TypeDB, and Qdrant. `Paper.source` is the PDF
provenance for the whole aggregate, while `Paper.graph` is the TypeDB-facing
metadata graph using relation-shaped structs such as authorings, affiliations,
and publications. `Paper.content`'s paragraph-level text chunks (plus the
abstract) are embedded and stored as Qdrant payloads.

## Flow

```text
PDF file or directory
    -> batch runner
    -> GROBID extraction
    -> TEI XML file
    -> TEI parser
    -> domain Paper
    -> exporters
        -> JSON file
        -> TypeDB store
        -> Qdrant store (chunks embedded via the configured OpenAI-compatible endpoint)
```

The entry point is configured through environment variables. `PDF_SOURCE`
accepts a single PDF or a directory of PDFs:

```sh
cargo run
```

`batch` discovers PDF inputs and runs the document pipeline concurrently. 
Each document writes the raw TEI XML first, then produces a normalized JSON representation of the domain model.

## Architecture

- `src/ingestion`: pipeline orchestration and document flow.
- `src/ingestion/extract`: external extraction sources
  - GROBID.
  - ROR (Research Organization Registry) client for resolving institution names to canonical organization ids.
- `src/ingestion/parse`: converts TEI XML into TEI structs.
- `src/ingestion/transform`: maps TEI structs into the domain model.
  - Embedding client for an OpenAI-compatible `/embeddings` endpoint, with retry/backoff on rate limiting.
- `src/ingestion/enrich`: post-transform, pre-export enrichment of the domain model
  - Resolves each institution's ROR id by name.
- `src/ingestion/export`: output sinks
  - JSON, TEI XML, TypeDB, and Qdrant are all wired today.
- `src/models`: TEI, path, and domain models.
- `src/stores`: database/service adapters with typed connected/disconnected
  states (Studio's PDF store is the exception: a stateless HTTP client with
  nothing to connect, so it's just constructed synchronously).

## Local Services

GROBID is required for the current PDF-to-JSON flow. Configure its endpoint with
`GROBID_URL`.
The TypeDB metadata schema lives at `schemas/typedb/domain.tql` and is exposed as
`database_builder_scepa_rs::stores::typedb::DOMAIN_SCHEMA` for store configuration.
Set `TYPEDB_WIPE_DATABASE=true` to delete and recreate the configured TypeDB
database before ingestion; the domain schema is applied after the recreate.

Every institution's name is matched against `ROR_HOST`'s affiliation-matching
endpoint (`ror.org`'s purpose-built endpoint for "large-scale programmatic
matching of complex, unstructured text strings to ROR IDs") and, on a match,
tagged with a `ror-id` attribute. This does not merge or deduplicate
institution entities itself — it only tags each one, so institutions that are
really the same organization but differ by typo or naming variant can be
merged later in a separate pass, keyed on `ror-id`, over already-exported
data. A failed or missing lookup is treated as no match and never fails the
paper's export.

The same pass also classifies each institution's `kind` (`university`,
`government-institution`, etc., per `schemas/typedb/domain.tql`). ROR's own
`types` field only gives a coarse `education`/`government`/... signal, so it's
used as a fallback; name keywords take precedence for distinctions ROR
doesn't make, such as university vs. university-of-applied-sciences
(hogeschool/Fachhochschule), or semi-government institutes (TNO, RIVM, ...)
that ROR has no equivalent category for. See
`src/ingestion/enrich/institution_kind.rs` for the exact keyword lists.

Chunk-level content lives in Qdrant payloads: the paper's abstract (if present)
and every non-empty paragraph-level text chunk are embedded via the
`OPENAI_HOST`/`OPENAI_API_KEY`/`OPENAI_EMBEDDING_MODEL`-configured endpoint and
upserted into the `QDRANT_COLLECTION` collection. Point ids are derived
deterministically from the paper's source hash and each chunk's locator, so
re-ingesting a paper updates its existing points instead of duplicating them.
Set `QDRANT_WIPE_COLLECTION=true` to delete and recreate the configured Qdrant
collection before ingestion.
Each payload carries `source`, `kind` (`abstract` or `section`), `section_index`,
`section_title`, `chunk_index`, and `text`.

Set `STUDIO_PDF_STORE_ENABLED=true` (plus `STUDIO_BASE_URL`/`STUDIO_API_KEY`) to
also push each paper's own PDF to a running Studio instance's
`/api/pdf/{sha256}` store, keyed by the same source hash used everywhere else
in this pipeline, so Studio can serve the original PDF for citations. This is
best-effort: Studio being unreachable is logged and does not fail the rest of
the paper's ingestion.

```sh
docker compose up -d
```

## Metadata server

A second binary, `server`, exposes GROBID-derived metadata over HTTP for
[upload_interface](../upload_interface)'s field-autocomplete form, independently
of the batch pipeline above:

```sh
cargo run --bin server
```

- `PUT /metadata/{sha256}` (multipart, field `file`): accepts a PDF, verifies
  its sha256 matches the path parameter, runs it through GROBID + TEI parsing
  + the same domain transform used by the batch pipeline, maps the result
  into upload_interface's `Field` schema, and returns it. Purely transient --
  nothing is written to disk here, so a document is never live-written to
  storage before the user has actually chosen to keep it.
- `GET /metadata/{sha256}`: retrieves a previously-*saved* result (404 if
  the document has only ever been extracted, never saved).
- `PATCH /metadata/{sha256}` (JSON body): persists the user's edited fields
  -- the only endpoint that writes to disk. Creates the record on its first
  call for a given hash, overwrites it on every call after.

All three routes require `Authorization: Bearer <key>`, checked against
`METADATA_API_KEYS` (the same `"app-name:key,other-app:key"` pattern used
elsewhere). Real user authentication (Authentik login) happens at the edge
in front of this server -- e.g. a Caddy reverse proxy with a
forward-auth/outpost integration -- not in this process itself; this
server only needs to trust the machine caller (the reverse proxy) that
reaches it directly.

This server never writes to TypeDB or Qdrant -- it only produces metadata
for a form to autocomplete from. Ingestion into TypeDB/Qdrant still only
happens when the batch pipeline (`cargo run`) is run, e.g. once a user saves
the document in upload_interface.

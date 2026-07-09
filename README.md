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
- `src/stores`: database adapters with typed connected/disconnected states.

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

```sh
docker compose up -d
```

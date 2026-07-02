# Database Builder SCEPA

Builds structured paper data from PDFs. The current pipeline extracts TEI XML with
GROBID, parses it into internal models, transforms it into the domain `Paper`
model, and exports JSON. The store layer is already present and is intended to
become part of the export pipeline.

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
        -> TypeDB store (planned)
        -> Qdrant store (planned)
```

The CLI entry point accepts a single PDF or a directory of PDFs:

```sh
cargo run -- <pdf-path-or-dir> <tei-xml-dir> <json-dir>
```

`batch` discovers PDF inputs and runs the document pipeline concurrently. 
Each document writes the raw TEI XML first, then produces a normalized JSON representation of the domain model.

## Architecture

- `src/ingestion`: pipeline orchestration and document flow.
- `src/ingestion/extract`: external extraction sources
  - GROBID.
- `src/ingestion/parse`: converts TEI XML into TEI structs.
- `src/ingestion/transform`: maps TEI structs into the domain model.
- `src/ingestion/export`: output sinks
  - JSON and TEI XML are wired today.
  - Stores are expected to join here.
- `src/models`: TEI, path, and domain models.
- `src/stores`: database adapters with typed connected/disconnected states.

## Local Services

GROBID is required for the current PDF-to-JSON flow and is expected at `http://localhost:8070`.
The TypeDB metadata schema lives at `schemas/typedb/domain.tql` and is exposed as
`database_builder_scepa_rs::stores::typedb::DOMAIN_SCHEMA` for store configuration.
Chunk-level content, embeddings, and bounding boxes are expected to live in Qdrant payloads.

```sh
docker compose up -d
```

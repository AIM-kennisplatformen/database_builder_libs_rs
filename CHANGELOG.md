## 0.2.0 (2026-07-02)

### Features

- **qdrant**: add qdrant connection support
- **typedb**: add typedb connection support
- **pipeline**: add pipeline multithreading support
- **cli**: add batch processing for directories
- **export**: add export pipeline from domain model to json
- **transformation**: add transformation pipeline from tei document to domain paper
- **transformation**: add domain models
- **parsing**: add parsing pipeline from tei xml to tei models
- **parsing**: add tei models
- **extraction**: add extraction of pdf files to tei xml through grobid

### Bug Fixes

- **parsing**: add support for ref and formula tags

### Refactoring

- **pipeline**: return specific pipeline error
- **export**: simplify export, move tei xml file creation to export

### Documentation

- **readme**: add flow and architecture readme

### Build

- **cargo**: add qdrant client
- **docker**: add qdrant service
- **cargo**: add typedb driver
- **docker**: add typedb service
- **cargo**: add parsing and export cargo dependencies
- **cargo**: bootstrap core cargo dependencies
- **docker**: add grobid service

### Chores

- **git**: add commitizen config
- **git**: configure lefthook for git hooks

use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    ingestion::{
        error::PipelineError,
        export::{
            json::{json_path_for_tei_xml, write_paper_json},
            qdrant::write_paper_qdrant,
            tei_xml::write_tei_xml,
            typedb::write_paper_typedb,
        },
        extract::grobid::source::GrobidSource,
        parse::tei::reader::parse_tei_xml_path,
        transform::{embedding::source::EmbeddingSource, tei::paper_from_tei},
    },
    models::{
        domain::SourceHash,
        paths::{pdf::PdfPath, tei_xml::TeiXmlPath},
    },
    stores::{
        qdrant::store::{QdrantConnected, QdrantStore},
        typedb::store::{TypedbConnected, TypedbStore},
    },
};

pub struct PipelineSources<'a> {
    pub grobid: &'a GrobidSource,
    pub embedding_source: &'a EmbeddingSource,
}

pub struct PipelineStores<'a> {
    pub typedb_store: &'a TypedbStore<TypedbConnected>,
    pub qdrant_store: &'a QdrantStore<QdrantConnected>,
}

pub async fn run_with_reporter<F>(
    pdf_path: PdfPath,
    tei_xml_dir: &Path,
    json_dir: &Path,
    sources: PipelineSources<'_>,
    stores: PipelineStores<'_>,
    mut report: F,
) -> Result<()>
where
    F: FnMut(&str),
{
    let PipelineSources {
        grobid,
        embedding_source,
    } = sources;
    let PipelineStores {
        typedb_store,
        qdrant_store,
    } = stores;

    let file_stem = pdf_path
        .file_stem()
        .ok_or_else(|| PipelineError::MissingPdfFileStem {
            path: pdf_path.as_path().to_path_buf(),
        })
        .with_context(|| format!("deriving output filenames from {}", pdf_path.display()))?;
    let tei_xml_path = TeiXmlPath::filename_from_stem(file_stem, tei_xml_dir);

    let pdf_bytes = match grobid
        .read_pdf(&pdf_path)
        .map_err(PipelineError::from)
        .with_context(|| format!("reading PDF from {}", pdf_path.display()))
    {
        Ok(pdf_bytes) => {
            report("Read PDF bytes");
            pdf_bytes
        }
        Err(error) => {
            report(&format!("Failed to read PDF: {error}"));
            return Err(error);
        }
    };
    let source = SourceHash::from_bytes(&pdf_bytes);
    report("Calculated PDF source hash");

    let tei_xml = match grobid
        .extract_pdf_bytes_to_tei_xml(&pdf_path, pdf_bytes)
        .await
        .map_err(PipelineError::from)
        .with_context(|| format!("extracting TEI XML from {}", pdf_path.display()))
    {
        Ok(tei_xml) => {
            report("Extracted TEI XML with GROBID");
            tei_xml
        }
        Err(error) => {
            report(&format!("Failed to parse PDF with GROBID: {error}"));
            return Err(error);
        }
    };

    write_tei_xml(&tei_xml_path, &tei_xml)
        .map_err(PipelineError::from)
        .with_context(|| format!("writing TEI XML to {}", tei_xml_path.display()))?;
    report(&format!("Saved TEI XML to {}", tei_xml_path.display()));

    let tei_document = match parse_tei_xml_path(&tei_xml_path)
        .map_err(PipelineError::from)
        .with_context(|| format!("parsing TEI XML from {}", tei_xml_path.display()))
    {
        Ok(document) => {
            report("Transformed TEI XML into tei document");
            document
        }
        Err(error) => {
            report(&format!("Failed to parse TEI XML: {error}"));
            return Err(error);
        }
    };

    let paper = paper_from_tei(&tei_document, source);
    report("Transformed tei document to domain paper");

    let json_path = json_path_for_tei_xml(&tei_xml_path, json_dir);
    if let Err(error) = write_paper_json(&paper, &json_path)
        .map_err(PipelineError::from)
        .with_context(|| format!("writing domain JSON to {}", json_path.display()))
    {
        report(&format!("Failed to export domain paper as JSON: {error}"));
        return Err(error);
    }

    report(&format!("Saved domain JSON to {}", json_path.display()));

    if let Err(error) = write_paper_typedb(&paper, typedb_store)
        .await
        .map_err(PipelineError::from)
        .context("writing domain paper to TypeDB")
    {
        report(&format!("Failed to export domain paper to TypeDB: {error}"));
        return Err(error);
    }

    report("Saved domain paper to TypeDB");

    if let Err(error) = write_paper_qdrant(&paper, embedding_source, qdrant_store)
        .await
        .map_err(PipelineError::from)
        .context("writing domain paper to Qdrant")
    {
        report(&format!("Failed to export domain paper to Qdrant: {error}"));
        return Err(error);
    }

    report("Saved domain paper to Qdrant");

    Ok(())
}

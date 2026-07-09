use crate::models::{
    domain::{
        Affiliation, Authoring, Citation, Department, Funding, Institution, InstitutionKind,
        Literature, LiteratureCore, Paper, PaperGraph, PdfExtractionData, Publication,
        PublicationDetails, ScientificLiterature, SourceHash,
    },
    tei::{bibliography::BiblStruct, document::TeiDocument},
};

use super::{
    authors::{ExtractedAffiliation, ExtractedAuthor, authors_from_tei},
    content::{citations_from_tei, document_content_from_tei},
    funding::fundings_from_tei,
    metadata::{literature_title_from_tei, paper_metadata_from_tei},
    publication::publication_details_from_tei,
};

pub fn paper_from_tei(document: &TeiDocument, source: SourceHash) -> Paper {
    let source_bibl = source_bibl(document);
    let publication = publication_details_from_tei(document, source_bibl);
    let mut metadata = paper_metadata_from_tei(document);
    metadata.journal.clone_from(&publication.journal);
    metadata.volume.clone_from(&publication.volume);
    metadata.issue.clone_from(&publication.issue);
    metadata.pages.clone_from(&publication.pages);

    let authors = authors_from_tei(document, source_bibl);
    let title = literature_title_from_tei(document, source_bibl);
    let citations = citations_from_tei(document);
    let fundings = fundings_from_tei(document);
    let content = document_content_from_tei(document);
    let graph = paper_graph_from_parts(title, &publication, authors, citations, fundings);

    Paper {
        source,
        graph,
        metadata,
        content,
        extraction_data: PdfExtractionData::default(),
    }
}

fn source_bibl(document: &TeiDocument) -> Option<&BiblStruct> {
    document
        .header
        .file_desc
        .source_desc
        .bibliographic_structures
        .first()
}

fn paper_graph_from_parts(
    title: Option<String>,
    publication: &PublicationDetails,
    extracted_authors: Vec<ExtractedAuthor>,
    citations: Vec<Citation>,
    fundings: Vec<Funding>,
) -> PaperGraph {
    let literature = Literature::Scientific(ScientificLiterature {
        core: LiteratureCore {
            title,
            publishing_date: publication.publishing_date.clone(),
            issn: publication.identifiers.issn.clone(),
            isbn: publication.identifiers.isbn.clone(),
        },
        doi: publication.identifiers.doi.clone(),
    });

    let publications = publication
        .publisher
        .as_deref()
        .map(|publisher| Publication {
            publisher: Institution {
                name: Some(publisher.to_owned()),
                kind: InstitutionKind::Institution,
                ror_id: None,
            },
        })
        .into_iter()
        .collect();

    let authorings = extracted_authors
        .into_iter()
        .map(authoring_from_extracted_author)
        .collect();

    PaperGraph {
        literature,
        authorings,
        publications,
        citations,
        fundings,
    }
}

fn authoring_from_extracted_author(extracted: ExtractedAuthor) -> Authoring {
    Authoring {
        author: extracted.author,
        affiliations: extracted
            .affiliations
            .into_iter()
            .filter_map(affiliation_from_extracted_affiliation)
            .collect(),
    }
}

fn affiliation_from_extracted_affiliation(extracted: ExtractedAffiliation) -> Option<Affiliation> {
    let institution_name = extracted.institution?;
    let department_name = extracted.department.or(extracted.laboratory);

    Some(Affiliation {
        institution: Institution {
            name: Some(institution_name),
            kind: InstitutionKind::Institution,
            ror_id: None,
        },
        department: department_name.map(|name| Department { name: Some(name) }),
    })
}

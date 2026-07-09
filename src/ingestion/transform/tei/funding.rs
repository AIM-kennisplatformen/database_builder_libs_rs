use crate::models::{
    domain::{Funding, Institution, InstitutionKind, Project},
    tei::{
        document::TeiDocument,
        header::Funder,
        text::{Block, Organization},
    },
};

use super::text::{normalized_opt, org_name_text};

/// Extracts funding statements from both places GROBID puts them: the
/// simple `titleStmt/funder` tag (just an organization, no grant id) and the
/// richer `listOrg[@type=funding]` blocks in the back matter (organization
/// plus `idno` grant/award identifiers). Institutions and projects built
/// here still need `enrich::ror` to resolve a ror-id/kind and are otherwise
/// exported as-is.
pub fn fundings_from_tei(document: &TeiDocument) -> Vec<Funding> {
    let mut fundings = Vec::new();

    for funder in &document.header.file_desc.title_stmt.funders {
        push_unique_funding(&mut fundings, funding_from_funder(funder));
    }

    if let Some(text) = document.text.as_ref()
        && let Some(back) = text.back.as_ref()
    {
        collect_fundings_from_blocks(&back.content, &mut fundings);
    }

    fundings
}

fn collect_fundings_from_blocks(blocks: &[Block], fundings: &mut Vec<Funding>) {
    for block in blocks {
        match block {
            Block::ListOrg(list_org) if list_org.typed.kind.as_deref() == Some("funding") => {
                for organization in &list_org.organizations {
                    push_unique_funding(fundings, funding_from_organization(organization));
                }
            }
            Block::Division(division) => {
                collect_fundings_from_blocks(&division.content, fundings);
            }
            _ => {}
        }
    }
}

/// GROBID's funding-extraction model emits the literal string "unknown" as
/// a placeholder org name when it detects a grant/funding mention but can't
/// identify the funder itself; treating that the same as "no name" avoids
/// creating junk institution entities from it.
fn is_placeholder_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("unknown")
}

fn funding_from_funder(funder: &Funder) -> Option<Funding> {
    let name = funder
        .organization_names
        .iter()
        .find_map(org_name_text)
        .or_else(|| normalized_opt(funder.text.as_deref()))
        .filter(|name| !is_placeholder_name(name))?;

    Some(Funding {
        funder: Institution {
            name: Some(name),
            kind: InstitutionKind::Institution,
            ror_id: None,
        },
        project: Project {
            name: None,
            number: None,
        },
    })
}

fn funding_from_organization(organization: &Organization) -> Option<Funding> {
    let name = organization
        .organization_names
        .iter()
        .find_map(org_name_text)
        .filter(|name| !is_placeholder_name(name))?;

    let number = organization
        .identifiers
        .iter()
        .filter(|idno| {
            idno.typed
                .kind
                .as_deref()
                .is_none_or(|kind| kind.to_lowercase().contains("grant"))
        })
        .find_map(|idno| normalized_opt(idno.text.as_deref()))
        .and_then(|text| text.parse::<i64>().ok());

    Some(Funding {
        funder: Institution {
            name: Some(name),
            kind: InstitutionKind::Institution,
            ror_id: None,
        },
        project: Project { name: None, number },
    })
}

fn push_unique_funding(fundings: &mut Vec<Funding>, funding: Option<Funding>) {
    let Some(funding) = funding else {
        return;
    };

    let is_duplicate = fundings.iter().any(|existing| {
        existing.funder.name == funding.funder.name
            && existing.project.number == funding.project.number
    });

    if !is_duplicate {
        fundings.push(funding);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(xml: &str) -> TeiDocument {
        quick_xml::de::from_str(xml).expect("fixture should be well-formed TEI")
    }

    #[test]
    fn extracts_a_simple_title_stmt_funder_with_no_grant_number() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt>
        <funder><orgName>National Science Foundation</orgName></funder>
      </titleStmt>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
</TEI>
"#,
        );

        let fundings = fundings_from_tei(&document);

        assert_eq!(fundings.len(), 1);
        assert_eq!(
            fundings[0].funder.name.as_deref(),
            Some("National Science Foundation")
        );
        assert_eq!(fundings[0].project.number, None);
    }

    #[test]
    fn extracts_an_organization_and_numeric_grant_number_from_the_back_matter() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org>
          <orgName>European Commission</orgName>
          <idno type="grant-number">731474</idno>
        </org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        let fundings = fundings_from_tei(&document);

        assert_eq!(fundings.len(), 1);
        assert_eq!(
            fundings[0].funder.name.as_deref(),
            Some("European Commission")
        );
        assert_eq!(fundings[0].project.number, Some(731474));
    }

    #[test]
    fn skips_grobids_unknown_funder_placeholder() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org>
          <orgName>unknown</orgName>
          <idno type="grant-number">12345</idno>
        </org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        assert!(fundings_from_tei(&document).is_empty());
    }

    #[test]
    fn leaves_the_project_number_absent_when_the_grant_identifier_is_not_numeric() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org>
          <orgName>Horizon 2020</orgName>
          <idno type="grant-number">H2020-ICT-2016-1</idno>
        </org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        let fundings = fundings_from_tei(&document);

        assert_eq!(fundings.len(), 1);
        assert_eq!(fundings[0].project.number, None);
    }

    #[test]
    fn ignores_identifiers_that_are_not_grant_numbers() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org>
          <orgName>Some Funder</orgName>
          <idno type="isni">0000000123456789</idno>
        </org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        let fundings = fundings_from_tei(&document);

        assert_eq!(fundings.len(), 1);
        assert_eq!(fundings[0].project.number, None);
    }

    #[test]
    fn finds_funding_lists_nested_inside_a_division() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <div type="acknowledgement">
        <listOrg type="funding">
          <org>
            <orgName>Nested Funder</orgName>
          </org>
        </listOrg>
      </div>
    </back>
  </text>
</TEI>
"#,
        );

        let fundings = fundings_from_tei(&document);

        assert_eq!(fundings.len(), 1);
        assert_eq!(fundings[0].funder.name.as_deref(), Some("Nested Funder"));
    }

    #[test]
    fn skips_organizations_with_no_name() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt/>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org><idno type="grant-number">12345</idno></org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        assert!(fundings_from_tei(&document).is_empty());
    }

    #[test]
    fn deduplicates_the_same_funder_seen_in_both_title_stmt_and_back_matter() {
        let document = document(
            r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt>
        <funder><orgName>Repeated Funder</orgName></funder>
      </titleStmt>
      <publicationStmt/>
      <sourceDesc/>
    </fileDesc>
  </teiHeader>
  <text>
    <back>
      <listOrg type="funding">
        <org><orgName>Repeated Funder</orgName></org>
      </listOrg>
    </back>
  </text>
</TEI>
"#,
        );

        assert_eq!(fundings_from_tei(&document).len(), 1);
    }
}

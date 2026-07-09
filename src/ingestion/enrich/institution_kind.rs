use crate::models::domain::InstitutionKind;

/// Institutes that are neither purely governmental nor purely independent
/// knowledge institutions (Dutch/German applied-research organizations funded
/// by government but operating independently). ROR has no equivalent
/// category, so this has to be a curated, name-matched list rather than
/// derived from ROR's own `types`. Expected to grow as more of these show up
/// in the corpus.
const SEMI_GOVERNMENT_KEYWORDS: &[&str] = &[
    "tno",
    "rivm",
    "deltares",
    "marin",
    "nlr",
    "rijksinstituut voor volksgezondheid",
];

const UNIVERSITY_OF_APPLIED_SCIENCES_KEYWORDS: &[&str] = &[
    "university of applied sciences",
    "universities of applied sciences",
    "hogeschool",
    "fachhochschule",
];

const UNIVERSITY_KEYWORDS: &[&str] = &[
    "university",
    "universiteit",
    "universität",
    "université",
    "universidad",
    "università",
];

/// Classifies an institution's kind from its extracted name and, if it was
/// matched, ROR's own `types` for that organization. Name keywords take
/// precedence over ROR's coarser `education`/`government` types so that,
/// e.g., a university of applied sciences (which ROR only tags as
/// `education`, same as a research university) isn't flattened into the
/// generic `knowledge-institution`.
pub fn classify_institution_kind(name: &str, ror_types: &[String]) -> InstitutionKind {
    let name = name.to_lowercase();

    if contains_any(&name, SEMI_GOVERNMENT_KEYWORDS) {
        return InstitutionKind::SemiGovernmentInstitution;
    }

    if contains_any(&name, UNIVERSITY_OF_APPLIED_SCIENCES_KEYWORDS) {
        return InstitutionKind::UniversityOfAppliedSciences;
    }

    if contains_any(&name, UNIVERSITY_KEYWORDS) {
        return InstitutionKind::University;
    }

    if has_ror_type(ror_types, "education") {
        return InstitutionKind::KnowledgeInstitution;
    }

    if has_ror_type(ror_types, "government") {
        return InstitutionKind::GovernmentInstitution;
    }

    InstitutionKind::Institution
}

fn contains_any(name: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| name.contains(keyword))
}

fn has_ror_type(ror_types: &[String], target: &str) -> bool {
    ror_types
        .iter()
        .any(|kind| kind.eq_ignore_ascii_case(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_a_semi_government_institute_regardless_of_ror_types() {
        assert_eq!(
            classify_institution_kind("TNO", &["company".to_owned()]),
            InstitutionKind::SemiGovernmentInstitution
        );
    }

    #[test]
    fn classifies_a_university_of_applied_sciences_over_the_generic_education_type() {
        assert_eq!(
            classify_institution_kind(
                "Fontys University of Applied Sciences",
                &["education".to_owned()]
            ),
            InstitutionKind::UniversityOfAppliedSciences
        );
    }

    #[test]
    fn classifies_a_hogeschool_as_a_university_of_applied_sciences() {
        assert_eq!(
            classify_institution_kind("Hogeschool Utrecht", &[]),
            InstitutionKind::UniversityOfAppliedSciences
        );
    }

    #[test]
    fn classifies_a_research_university_by_name() {
        assert_eq!(
            classify_institution_kind("University of Bremen", &["education".to_owned()]),
            InstitutionKind::University
        );
    }

    #[test]
    fn classifies_a_research_university_by_name_even_without_a_ror_match() {
        assert_eq!(
            classify_institution_kind("Delft University of Technology", &[]),
            InstitutionKind::University
        );
    }

    #[test]
    fn falls_back_to_the_generic_education_kind_when_no_name_keyword_matches() {
        assert_eq!(
            classify_institution_kind("Institute for Advanced Study", &["education".to_owned()]),
            InstitutionKind::KnowledgeInstitution
        );
    }

    #[test]
    fn classifies_a_government_body_from_ror_types() {
        assert_eq!(
            classify_institution_kind("Ministry of Health", &["government".to_owned()]),
            InstitutionKind::GovernmentInstitution
        );
    }

    #[test]
    fn falls_back_to_the_generic_institution_kind_with_no_signal() {
        assert_eq!(
            classify_institution_kind("Leuze electronic", &[]),
            InstitutionKind::Institution
        );
    }
}

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Institution {
    pub name: Option<String>,
    pub kind: InstitutionKind,
    /// The institution's ROR (Research Organization Registry) id, resolved
    /// by name against ror.org. Tags the institution for later
    /// deduplication of near-duplicate entities (typos, naming variants)
    /// that share the same organization; merging on this id is a separate,
    /// later step, not something this field does by itself.
    pub ror_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstitutionKind {
    Institution,
    GovernmentInstitution,
    SemiGovernmentInstitution,
    KnowledgeInstitution,
    University,
    UniversityOfAppliedSciences,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Department {
    pub name: Option<String>,
}

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Institution {
    pub name: Option<String>,
    pub kind: InstitutionKind,
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

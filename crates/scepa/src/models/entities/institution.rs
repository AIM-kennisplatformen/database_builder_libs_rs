use serde::{Deserialize, Serialize};

use crate::models::relations::publication_event::Publisher;
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum InstitutionEntity {
    Institution(Institution),
    GovernmentInstitution(GovernmentInstitution),
    EducationInstitution(EducationInstitution),
    NonprofitInstitution(NonprofitInstitution),
}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Institution {
    pub entity_id: String,
    pub ror: Option<String>,
}

#[typedb_relation_role(name = "institution")]
impl Publisher for Institution {}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GovernmentInstitution {
    pub entity_id: String,
    pub ror: Option<String>,
}

#[typedb_relation_role(name = "government-institution")]
impl Publisher for GovernmentInstitution {}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EducationInstitution {
    pub entity_id: String,
    pub ror: Option<String>,
}

#[typedb_relation_role(name = "education-institution")]
impl Publisher for EducationInstitution {}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NonprofitInstitution {
    pub entity_id: String,
    pub ror: Option<String>,
}

#[typedb_relation_role(name = "nonprofit-institution")]
impl Publisher for NonprofitInstitution {}

use serde::{Deserialize, Serialize};

use crate::models::relations::publishing::Publisher;
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub enum InstitutionEntity {
    Institution(Institution),
    GovernmentInstitution(GovernmentInstitution),
    EducationInstitution(EducationInstitution),
    NonprofitInstitution(NonprofitInstitution),
}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct Institution {}

#[typedb_relation_role(name = "institution")]
impl Publisher for Institution {}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct GovernmentInstitution {}

#[typedb_relation_role(name = "government-institution")]
impl Publisher for GovernmentInstitution {}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct EducationInstitution {}

#[typedb_relation_role(name = "education-institution")]
impl Publisher for EducationInstitution {}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct NonprofitInstitution {}

#[typedb_relation_role(name = "nonprofit-institution")]
impl Publisher for NonprofitInstitution {}

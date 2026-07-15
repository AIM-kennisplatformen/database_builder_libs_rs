use serde::{Deserialize, Serialize};

use crate::models::relations::contribution::Work;
use crate::models::typedb_relation;

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Citation {
    pub citing: Box<dyn Work>,
    pub cited: Box<dyn Work>,
}

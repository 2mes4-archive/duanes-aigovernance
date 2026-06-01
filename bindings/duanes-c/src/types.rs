use std::sync::Arc;

pub struct DuanesEngine {
    pub router: Arc<duanes_core::InferenceRouter>,
    pub storage: Arc<duanes_storage::SqliteStorage>,
    pub config: Arc<duanes_common::DuanesConfig>,
}

use reader_core::ReaderCore;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub core: Arc<ReaderCore>,
}

use super::container_status::ContainerStatusWatcherTrait;
use std::sync::Arc;

#[derive(Clone)]
pub struct Watchers {
    pub container_status_watcher: Arc<dyn ContainerStatusWatcherTrait + Send + Sync>,
}

impl Watchers {
    pub fn new(
        container_status_watcher: Arc<dyn ContainerStatusWatcherTrait + Send + Sync>,
    ) -> Self {
        Watchers {
            container_status_watcher,
        }
    }
}

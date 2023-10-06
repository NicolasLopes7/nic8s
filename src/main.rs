mod entities;
mod watchers;
use std::{collections::HashMap, sync::Arc, thread, time::Duration};

use tokio::{sync::Mutex, task};
use watchers::{container_status::ContainerStatusWatcher, watchers::Watchers};

use crate::entities::container::Container;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let status_watcher = Arc::new(ContainerStatusWatcher {
        containers: Arc::new(Mutex::new(HashMap::new())),
    });
    let watchers = Watchers::new(status_watcher.clone());

    Container::new("nginx", "80", "nginx", &status_watcher).await?;

    let clone_watchers = watchers.clone();
    let container_status_checker_task = task::spawn(async move {
        loop {
            clone_watchers.container_status_watcher.check_status().await;
            thread::sleep(Duration::from_secs(1))
        }
    });

    container_status_checker_task.await?;
    Ok(())
}

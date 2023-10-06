use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::{process::Command, sync::Mutex};

use crate::entities::container::{Container, ContainerStatus};

pub struct ContainerStatusWatcher {
    pub containers: Arc<Mutex<HashMap<String, ContainerStatus>>>,
}

#[async_trait]
pub trait ContainerStatusWatcherTrait {
    async fn check_status(&self);
}

#[async_trait]
impl ContainerStatusWatcherTrait for ContainerStatusWatcher {
    async fn check_status(&self) {
        println!("Checking status");
        let containers = self.containers.lock();

        for (id, status) in containers.await.iter_mut() {
            println!(
                "Checking status for container: {}\nCurrent status is: {:?}\n------------------",
                id,
                status.clone()
            );
            let mut command = Command::new("docker");

            command
                .arg("inspect")
                .arg("--format")
                .arg("{{.State.Status}}")
                .arg(id);

            let out = command.output().await.unwrap();

            if out.status.success() {
                let new_container_status = self.container_status_mapper(
                    String::from_utf8_lossy(&out.stdout).trim().to_string(),
                );

                if new_container_status != status.clone() {
                    *status = new_container_status
                }
            }
        }
    }
}

impl ContainerStatusWatcher {
    fn new() -> Self {
        ContainerStatusWatcher {
            containers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_container(&self, container: Container) {
        self.containers
            .lock()
            .await
            .insert(container.clone().id, container.get_status());
    }

    fn container_status_mapper(&self, status: String) -> ContainerStatus {
        match status.as_str() {
            "created" => ContainerStatus::Created,
            "running" => ContainerStatus::Running,
            "restarting" => ContainerStatus::Restarting,
            "exited" => ContainerStatus::Exited,
            "paused" => ContainerStatus::Paused,
            "dead" => ContainerStatus::Dead,
            _ => ContainerStatus::Unknown,
        }
    }
}

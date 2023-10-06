use anyhow::{anyhow, Ok};
use tokio::process::Command;

use crate::watchers::{container_status::ContainerStatusWatcher, watchers::Watchers};

#[derive(Clone, PartialEq, Debug)]
pub enum ContainerStatus {
    Created,
    Running,
    Restarting,
    Exited,
    Paused,
    Dead,
    Unknown,
}

#[derive(Clone)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub created: String,
    pub ports: String,
    status: &'static ContainerStatus,
}

impl Container {
    pub async fn new(
        name: &str,
        ports: &str,
        image: &str,
        status_watcher: &ContainerStatusWatcher,
    ) -> Result<Container, anyhow::Error> {
        let mut command = Command::new("docker");

        command
            .arg("run")
            .arg("-d")
            .arg("--name")
            .arg(String::from(name))
            .arg("-p")
            .arg(String::from(ports))
            .arg(String::from(image));

        let out = command.output().await?;

        if !out.status.success() {
            return Err(anyhow!(
                "failed to execute process: {}\n{}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        let container_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        println!("Container ID: {}", container_id);
        let container = Container {
            id: container_id,
            name: String::from(name),
            image: String::from(image),
            created: chrono::Local::now().to_string(),
            ports: String::from(ports),
            status: &ContainerStatus::Created,
        };

        status_watcher.add_container(container.clone()).await;
        Ok(container)
    }

    pub fn get_status(&self) -> ContainerStatus {
        self.status.clone()
    }
}

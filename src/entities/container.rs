use anyhow::anyhow;
use tokio::process::Command;

pub enum ContainerStatus {
    Created,
    Running,
    Error,
}

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
            return Err(anyhow!("failed to execute process: {}", out.status));
        }

        let container_id = String::from_utf8_lossy(&out.stdout).trim().to_string();

        Ok(Container {
            id: container_id,
            name: String::from(name),
            image: String::from(image),
            created: chrono::Local::now().to_string(),
            ports: String::from(ports),
            status: &ContainerStatus::Created,
        })
    }
}

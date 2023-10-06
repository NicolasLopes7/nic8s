mod entities;
use crate::entities::container::Container;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    Container::new("nginx", "80:80", "nginx").await?;

    Ok(())
}

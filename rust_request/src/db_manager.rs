use anyhow::Result;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use rand::Rng;

pub const TABLE_NAME: &str = "rust-cache";
pub struct DBManager {
    client: Client,
    job_id: String,
}
impl DBManager {
    pub async fn init(job_id: String) -> Self {
        let config = aws_config::load_from_env().await;
        Self {
            client: Client::new(&config),
            job_id,
        }
    }
    pub async fn send_result(self, result: String) -> Result<()> {
        self.client
            .put_item()
            .table_name(TABLE_NAME)
            .item("job_id", AttributeValue::S(self.job_id))
            .item("job_state", AttributeValue::S("100".to_string()))
            .item("job_result", AttributeValue::S(result))
            .send()
            .await?;
        Ok(())
    }
}

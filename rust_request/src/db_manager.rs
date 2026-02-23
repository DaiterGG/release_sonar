use anyhow::Result;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use rand::Rng;

pub const TABLE_NAME: &str = "result-cache";
pub struct DBManager {
    client: Client,
    job_id: String
}
impl DBManager {
    pub async fn init() -> Self {
        let config = aws_config::load_from_env().await;
        let job_id = rand::rng().next_u64().to_string();
         Self {client: Client::new(&config), job_id }
    }
    pub async fn send_result(self, result: String)-> Result<()> {
        self.client
            .put_item()
            .table_name(TABLE_NAME)
            .item("job_id", AttributeValue::N(self.job_id))
            .item("job_state", AttributeValue::N("100".to_string()))
            .item("job_result", AttributeValue::N(result))
            .send()
            .await?;
        Ok(())
    }
}

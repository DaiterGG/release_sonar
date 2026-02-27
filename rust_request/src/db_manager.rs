use anyhow::Result;
use aws_sdk_dynamodb::{Client, types::AttributeValue};

pub const TABLE_NAME: &str = "rust-cache";
pub struct DBManager {
    client: Client,
    job_code: String,
    job_stamp: String,
}

pub trait SendProgress {
    async fn send(&self, progress: i32);
}
impl DBManager {
    pub async fn init(job_code: &str, job_stamp: String) -> Self {
        let config = aws_config::load_from_env().await;
        Self {
            client: Client::new(&config),
            job_code: job_code.to_string(),
            job_stamp,
        }
    }
    pub async fn send_result(self, result: String) -> Result<()> {
        self.client
            .put_item()
            .table_name(TABLE_NAME)
            .item("job_code", AttributeValue::S(self.job_code))
            .item("job_stamp", AttributeValue::N(self.job_stamp))
            .item("job_state", AttributeValue::S("DONE".to_string()))
            .item("job_result", AttributeValue::S(result))
            .send()
            .await?;
        Ok(())
    }
}

impl SendProgress for DBManager {
    async fn send(&self, progress: i32) {
        let res = self
            .client
            .put_item()
            .table_name(TABLE_NAME)
            .item("job_code", AttributeValue::S(self.job_code.clone()))
            .item("job_stamp", AttributeValue::N(self.job_stamp.clone()))
            .item("job_state", AttributeValue::S("PROGRESS".to_string()))
            .item("job_result", AttributeValue::S(progress.to_string()))
            .send()
            .await;
        if let Err(e) = res {
            println!("db send err: {e}");
        }
    }
}

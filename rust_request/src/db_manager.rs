use anyhow::Result;
use aws_sdk_dynamodb::{Client, types::AttributeValue};

pub const TABLE_NAME: &str = "rust_cache";
pub struct DBManager {
    client: Client,
    job_code: String,
    job_expire_time: String,
}

pub trait SendProgress {
    async fn send(&self, progress: i32);
}
impl DBManager {
    pub async fn init(auth_code: &str, expire_time: String) -> Self {
        let config = aws_config::load_from_env().await;
        let _check: u64 = expire_time.parse().expect("should be a number");
        Self {
            client: Client::new(&config),
            job_code: auth_code.to_string(),
            job_expire_time: expire_time,
        }
    }
    pub async fn send_result(self, result: String) -> Result<()> {
        self.client
            .put_item()
            .table_name(TABLE_NAME)
            .item("job_auth_code", AttributeValue::S(self.job_code))
            .item("job_expire_time", AttributeValue::N(self.job_expire_time))
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
            .item("job_auth_code", AttributeValue::S(self.job_code.clone()))
            .item(
                "job_expire_time",
                AttributeValue::N(self.job_expire_time.clone()),
            )
            .item("job_state", AttributeValue::S("PROGRESS".to_string()))
            .item("job_result", AttributeValue::S(progress.to_string()))
            .send()
            .await;
        if let Err(e) = res {
            println!("db send err: {e}");
        }
    }
}

// 导入必要的外部依赖
use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde_json::{json, Value};
use std::fmt;

// 定义 protobuf 生成的模块
pub mod proto {
    pub mod block_engine {
        tonic::include_proto!("block_engine");
    }

    pub mod bundle {
        tonic::include_proto!("bundle");
    }

    pub mod packet {
        tonic::include_proto!("packet");
    }

    pub mod shared {
        tonic::include_proto!("shared");
    }

    pub mod searcher {
        tonic::include_proto!("searcher");
    }
}

// 导出 gRPC 模块
pub mod grpc;
pub use grpc::GrpcClient;

// JSON-RPC SDK 实现
pub struct JitoJsonRpcSDK {
    base_url: String,                // API 基础 URL
    uuid: Option<String>,            // 可选的 UUID
    client: Client,                  // HTTP 客户端
    grpc_url: Option<String>,        // gRPC URL
}

#[derive(Debug)]
pub struct PrettyJsonValue(pub Value);

impl fmt::Display for PrettyJsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string_pretty(&self.0).unwrap())
    }
}

impl From<Value> for PrettyJsonValue {
    fn from(value: Value) -> Self {
        PrettyJsonValue(value)
    }
}

impl JitoJsonRpcSDK {
    // 创建新的 SDK 实例
    pub fn new(base_url: &str, uuid: Option<String>) -> Self {
        Self {
            base_url: base_url.to_string(),
            uuid,
            client: Client::new(),
            grpc_url: None,
        }
    }


    // 发送 JSON-RPC 请求的通用方法
    async fn send_request(
        &self,
        endpoint: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, reqwest::Error> {
        let url = format!("{}{}", self.base_url, endpoint);

        let data = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or(json!([]))
        });

        println!("Sending request to: {}", url);
        println!(
            "Request body: {}",
            serde_json::to_string_pretty(&data).unwrap()
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);

        let body = response.json::<Value>().await?;
        println!(
            "Response body: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );

        Ok(body)
    }

    // 获取随机的 tip account
    pub async fn get_random_tip_account(&self) -> Result<String> {
        let tip_accounts_response = self.get_tip_accounts().await?;

        let tip_accounts = tip_accounts_response["result"]
            .as_array()
            .ok_or_else(|| anyhow!("Failed to parse tip accounts as array"))?;

        if tip_accounts.is_empty() {
            return Err(anyhow!("No tip accounts available"));
        }

        let random_account = tip_accounts
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| anyhow!("Failed to choose random tip account"))?;

        random_account
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse tip account as string"))
            .map(String::from)
    }

    pub async fn get_bundle_statuses(&self, bundle_uuids: Vec<String>) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        let params = json!([bundle_uuids]);

        self.send_request(&endpoint, "getBundleStatuses", Some(params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    pub async fn get_tip_accounts(&self) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
    }

   

    pub fn prettify(value: Value) -> PrettyJsonValue {
        PrettyJsonValue(value)
    }

    // 校验 bundle 参数
    fn validate_bundle_params(params: &Option<Value>) -> Result<()> {
        match params {
            Some(Value::Array(outer_array)) if outer_array.len() >= 1 => {
                if let Some(serialized_txs) = outer_array[0].as_array() {
                    if serialized_txs.is_empty() {
                        return Err(anyhow!("Bundle must contain at least one transaction"));
                    }
                    if serialized_txs.len() > 5 {
                        return Err(anyhow!("Bundle can contain at most 5 transactions"));
                    }
                    Ok(())
                } else {
                    Err(anyhow!("First element must be an array of transactions"))
                }
            }
            _ => Err(anyhow!(
                "Invalid bundle format: expected [serialized_txs, options]"
            )),
        }
    }

    // 发送交易包
    pub async fn send_bundle(
        &self,
        params: Option<Value>,
        uuid: Option<&str>,
    ) -> Result<Value, anyhow::Error> {
        let mut endpoint = "/bundles".to_string();

        if let Some(uuid) = uuid {
            endpoint = format!("{}?uuid={}", endpoint, uuid);
        }

        // 先进行参数校验
        Self::validate_bundle_params(&params)?;

        self.send_request(&endpoint, "sendBundle", params)
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

  
}

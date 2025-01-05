// 导入必要的外部依赖
use tonic::transport::{Channel, Error as TransportError, Uri};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::fmt;
use rand::seq::SliceRandom;

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
pub use proto::block_engine::block_engine_validator_client::BlockEngineValidatorClient as BlockEngineClient;
pub use proto::bundle::{Bundle, BundleResult};
pub use proto::searcher::searcher_service_client::SearcherServiceClient;
pub use proto::searcher::{GetTipAccountsRequest, GetTipAccountsResponse};

// gRPC 客户端封装
#[derive(Debug)]
pub struct GrpcClient {
    client: SearcherServiceClient<Channel>,
}

impl GrpcClient {
    // 创建新的 gRPC 连接
    pub async fn connect(addr: &str) -> Result<Self> {
        let uri = addr.parse::<Uri>().unwrap();
        let channel = Channel::builder(uri)
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;
            
        let client = SearcherServiceClient::new(channel);
        Ok(Self { client })
    }

    // 获取 tip accounts 列表
    pub async fn get_tip_accounts(&mut self) -> Result<Vec<String>> {
        let request = tonic::Request::new(GetTipAccountsRequest {});
        
        match self.client.get_tip_accounts(request).await {
            Ok(response) => {
                let response = response.into_inner();
                Ok(response.accounts)
            }
            Err(e) => Err(anyhow!("Failed to get tip accounts: {}", e))
        }
    }
}

// JSON-RPC SDK 实现
pub struct JitoJsonRpcSDK {
    base_url: String,      // API 基础 URL
    uuid: Option<String>,  // 可选的 UUID
    client: Client,        // HTTP 客户端
    grpc_client: Option<GrpcClient>, // 可选的 gRPC 客户端
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
            grpc_client: None,
        }
    }

    // 使用 gRPC 创建新的 SDK 实例
    pub async fn new_with_grpc(
        base_url: &str,
        uuid: Option<String>,
        grpc_url: &str,
    ) -> Result<Self> {
        Ok(Self {
            base_url: base_url.to_string(),
            uuid,
            client: Client::new(),
            grpc_client: Some(GrpcClient::connect(grpc_url).await?),
        })
    }

    pub async fn enable_grpc(&mut self, grpc_url: &str) -> Result<()> {
        self.grpc_client = Some(GrpcClient::connect(grpc_url).await?);
        Ok(())
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


    pub async fn get_tip_accounts(&self) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
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

        let transactions = match params {
            Some(Value::Array(outer_array)) if outer_array.len() >= 1 => {
                if let Some(serialized_txs) = outer_array[0].as_array() {
                    if serialized_txs.is_empty() {
                        return Err(anyhow!("Bundle must contain at least one transaction"));
                    }
                    if serialized_txs.len() > 5 {
                        return Err(anyhow!("Bundle can contain at most 5 transactions"));
                    }
                    outer_array
                } else {
                    return Err(anyhow!("First element must be an array of transactions"));
                }
            }
            _ => {
                return Err(anyhow!(
                    "Invalid bundle format: expected [serialized_txs, options]"
                ))
            }
        };

        self.send_request(&endpoint, "sendBundle", Some(json!(transactions)))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    // pub async fn send_bundle_with_grpc(
    //     &self,
    //     params: Option<Value>,
    //     uuid: Option<&str>,
    // ) -> Result<Value> {
    //     let client = self.grpc_client
    //         .as_ref()
    //         .ok_or_else(|| anyhow!("gRPC not enabled. Call enable_grpc() first"))?;

    //     let transactions = match params {
    //         Some(Value::Array(outer_array)) if outer_array.len() >= 1 => {
    //             if let Some(serialized_txs) = outer_array[0].as_array() {
    //                 if serialized_txs.is_empty() {
    //                     return Err(anyhow!("Bundle must contain at least one transaction"));
    //                 }
    //                 if serialized_txs.len() > 5 {
    //                     return Err(anyhow!("Bundle can contain at most 5 transactions"));
    //                 }
    //                 outer_array[0].clone()
    //             } else {
    //                 return Err(anyhow!("First element must be an array of transactions"));
    //             }
    //         }
    //         _ => return Err(anyhow!("Invalid bundle format: expected [serialized_txs, options]")),
    //     };

    //     client.send_bundle(transactions, uuid.map(String::from)).await
    // }


    // 新增 GRPC 版本的 get_tip_accounts 方法
    pub async fn get_tip_accounts_grpc(&mut self) -> Result<Value> {
        let grpc_client = self.grpc_client
            .as_mut()
            .ok_or_else(|| anyhow!("gRPC not enabled. Call enable_grpc() first"))?;
            
        let accounts = grpc_client.get_tip_accounts().await?;
        
        Ok(json!(accounts))
    }

    pub fn prettify(value: Value) -> PrettyJsonValue {
        PrettyJsonValue(value)
    }

}
use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde_json::{json, Value};
use std::fmt;
use tonic::transport::Channel;

#[derive(Debug)]
pub struct GrpcClient {
    channel: Channel,
}

// 连接到 gRPC 服务器
impl GrpcClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let channel = Channel::from_shared(addr.to_string())?.connect().await?;
        Ok(Self { channel })
    }

    pub async fn send_bundle(&self, transactions: Value) -> Result<Value> {
        let mut client = tonic::client::Grpc::new(self.channel.clone());
        let request = tonic::Request::new(transactions.to_string());

        let path = tonic::codegen::http::uri::PathAndQuery::from_static("/api/v1/bundles");
        let codec = tonic::codec::ProstCodec::<String, String>::default();

        let response = client.unary(request, path, codec).await?;
        let data = response.into_inner();
        serde_json::from_str(&data).map_err(|e| anyhow!("Failed to parse response: {}", e))
    }
}

pub struct JitoJsonRpcSDK {
    base_url: String,
    uuid: Option<String>,
    client: Client,
    grpc_client: Option<GrpcClient>,
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
    pub fn new(base_url: &str, uuid: Option<String>) -> Self {
        Self {
            base_url: base_url.to_string(),
            uuid,
            client: Client::new(),
            grpc_client: None,
        }
    }

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

    async fn send_request_grpc(
        &mut self,
        endpoint: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        let client = self
            .grpc_client
            .as_mut()
            .ok_or_else(|| anyhow!("gRPC not enabled. Call enable_grpc() first"))?;

        let mut grpc = tonic::client::Grpc::new(client.channel.clone());
        let request = tonic::Request::new(
            serde_json::json!({
                "endpoint": endpoint,
                "method": method,
                "params": params.unwrap_or(json!([]))
            })
            .to_string(),
        );

        let response = grpc
            .unary(
                request,
                tonic::codegen::http::uri::PathAndQuery::from_static(
                    "/jito.JitoService/SendRequest",
                ),
                tonic::codec::ProstCodec::<String, String>::default(),
            )
            .await?;

        let data = response.into_inner();
        serde_json::from_str(&data).map_err(|e| anyhow!("Failed to parse response: {}", e))
    }

    pub async fn get_tip_accounts(&self) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
    }

    pub async fn get_tip_accounts_with_grpc(&mut self) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request_grpc(&endpoint, "getTipAccounts", None)
            .await
    }

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

    pub async fn get_bundle_statuses_with_grpc(
        &mut self,
        bundle_uuids: Vec<String>,
    ) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        let params = json!([bundle_uuids]);
        self.send_request_grpc(&endpoint, "getBundleStatuses", Some(params))
            .await
    }

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

    pub async fn send_bundle_with_grpc(
        &self,
        params: Option<Value>,
        uuid: Option<&str>,
    ) -> Result<Value> {
        let client = self.grpc_client
            .as_ref()
            .ok_or_else(|| anyhow!("gRPC not enabled. Call enable_grpc() first"))?;

        // 验证参数格式
        let transactions = match params {
            Some(Value::Array(outer_array)) if outer_array.len() >= 1 => {
                if let Some(serialized_txs) = outer_array[0].as_array() {
                    if serialized_txs.is_empty() {
                        return Err(anyhow!("Bundle must contain at least one transaction"));
                    }
                    if serialized_txs.len() > 5 {
                        return Err(anyhow!("Bundle can contain at most 5 transactions"));
                    }
                    outer_array[0].clone()
                } else {
                    return Err(anyhow!("First element must be an array of transactions"));
                }
            }
            _ => return Err(anyhow!("Invalid bundle format: expected [serialized_txs, options]")),
        };

        client.send_bundle(transactions).await
    }

    pub fn prettify(value: Value) -> PrettyJsonValue {
        PrettyJsonValue(value)
    }
}

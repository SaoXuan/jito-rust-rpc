use anyhow::{anyhow, Result};
use crate::proto::packet::Packet as ProtoPacket;
use crate::proto::bundle::{Bundle};
use crate::proto::searcher::{
    GetTipAccountsRequest, GetTipAccountsResponse, SendBundleRequest,
};
use crate::proto::searcher::searcher_service_client::SearcherServiceClient;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;
use tonic::transport::{Channel, Endpoint};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct GrpcClient {
    client: Arc<Mutex<SearcherServiceClient<Channel>>>,
}

impl GrpcClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let endpoint = if addr.contains("https") {
            Endpoint::from_shared(addr.to_string())
                .expect("invalid url")
                .tls_config(tonic::transport::ClientTlsConfig::new())
                .expect("tls config error")
                .connect_timeout(std::time::Duration::from_secs(10))
                .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
                .http2_keep_alive_interval(std::time::Duration::from_secs(30))
                .keep_alive_timeout(std::time::Duration::from_secs(20))
                .keep_alive_while_idle(true)
        } else {
            Endpoint::from_shared(addr.to_string())
                .expect("invalid url")
                .connect_timeout(std::time::Duration::from_secs(10))
                .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
                .http2_keep_alive_interval(std::time::Duration::from_secs(30))
                .keep_alive_timeout(std::time::Duration::from_secs(20))
                .keep_alive_while_idle(true)
        };

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        let client = SearcherServiceClient::new(channel);
        Ok(Self { 
            client: Arc::new(Mutex::new(client))
        })
    }

    pub async fn get_tip_accounts(&self) -> Result<GetTipAccountsResponse> {
        let request = tonic::Request::new(GetTipAccountsRequest {});
        let mut client = self.client.lock().await;
        match client.get_tip_accounts(request).await {
            Ok(response) => Ok(response.into_inner()),
            Err(e) => Err(anyhow!("Failed to get tip accounts: {}", e)),
        }
    }

    pub async fn send_bundle(&self, transactions: Vec<VersionedTransaction>) -> Result<String> {
        println!("准备发送 bundle，交易数量: {}", transactions.len());
        
        let request = tonic::Request::new(SendBundleRequest {
            bundle: Some(Bundle {
                header: None,
                packets: transactions
                    .iter()
                    .map(proto_packet_from_versioned_tx)
                    .collect(),
            }),
        });
        
        println!("已创建 gRPC 请求");
        match self.client.lock().await.send_bundle(request).await {
            Ok(response) => {
                let uuid = response.into_inner().uuid;
                println!("Bundle 发送成功，UUID: {}", uuid);
                Ok(uuid)
            },
            Err(e) => {
                println!("Bundle 发送失败: {}", e);
                Err(anyhow!("Failed to send bundle: {}", e))
            },
        }
    }
}

fn proto_packet_from_versioned_tx(tx: &VersionedTransaction) -> ProtoPacket {
    ProtoPacket {
        data: bincode::serialize(&tx).unwrap(),
        meta: None,
    }
} 
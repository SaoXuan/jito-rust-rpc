use jito_sdk_rust::{GrpcClient, JitoJsonRpcSDK, proto::searcher::GetTipAccountsRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    get_tip_accounts().await?;
    Ok(())
} 

pub async fn get_tip_accounts() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 Jito 的服务端点
    let mut client = GrpcClient::connect("https://mainnet.block-engine.jito.wtf").await?;
    
    // 获取小费账户
    match client.get_tip_accounts().await {
        Ok(response) => {
            println!("获取到的小费账户列表:");
            for (index, account) in response.accounts.iter().enumerate() {
                println!("{}. {}", index + 1, account);
            }
        },
        Err(e) => {
            eprintln!("获取小费账户失败: {}", e);
        }
    }
    
    Ok(())
}

pub async fn get_tip_accounts_by_sdk() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 Jito 的服务端点
    let mut sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf", None);
   
   
    // 获取小费账户
    match sdk.get_tip_accounts_grpc().await {
        Ok(response) => {
            println!("获取到的小费账户列表:");
            if let Some(accounts) = response["result"].as_array() {
                for (index, account) in accounts.iter().enumerate() {
                    println!("{}. {}", index + 1, account);
                }
            }
        },
        Err(e) => {
            eprintln!("获取小费账户失败: {}", e);
        }
    }
    
    Ok(())
}
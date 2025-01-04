use jito_sdk_rust::JitoJsonRpcSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 SDK 实例
    let mut sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);
    
    // 使用原有的 RPC 方法
    println!("使用原有的 RPC 方法:");
    match sdk.get_tip_accounts().await {
        Ok(tip_accounts) => {
            let pretty_tip_accounts = JitoJsonRpcSDK::prettify(tip_accounts);
            println!("Tip accounts (RPC):\n{}", pretty_tip_accounts);
        },
        Err(e) => eprintln!("RPC Error: {:?}", e),
    }

    // 启用 gRPC 支持
    sdk.enable_grpc("http://localhost:50051").await?;
    
    // 使用新的 gRPC 方法
    println!("\n使用 gRPC 方法:");
    match sdk.get_tip_accounts_with_grpc().await {
        Ok(tip_accounts) => {
            println!("Tip accounts (gRPC):\n{}", serde_json::to_string_pretty(&tip_accounts)?);
        },
        Err(e) => eprintln!("gRPC Error: {:?}", e),
    }

    // 继续使用原有的 RPC 方法
    println!("\n继续使用原有的 RPC 方法:");
    match sdk.get_random_tip_account().await {
        Ok(account) => println!("Random tip account (RPC): {}", account),
        Err(e) => eprintln!("RPC Error: {:?}", e),
    }

    // 使用新的 gRPC 方法
    println!("\n使用新的 gRPC 方法:");
    match sdk.get_random_tip_account_with_grpc().await {
        Ok(account) => println!("Random tip account (gRPC): {}", account),
        Err(e) => eprintln!("gRPC Error: {:?}", e),
    }

    Ok(())
}
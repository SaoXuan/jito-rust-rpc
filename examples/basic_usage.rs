use jito_sdk_rust::JitoJsonRpcSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 SDK 实例
    let mut sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);
    
    // 使用原有的 RPC 方法
    println!("使用原有的 RPC 方法:");
    match sdk.get_tip_accounts().await {
        Ok(tip_accounts) => {
            println!("Tip accounts (RPC):\n{}", serde_json::to_string_pretty(&tip_accounts)?);
        },
        Err(e) => eprintln!("RPC Error: {:?}", e),
    }

    // 启用 gRPC 支持
    sdk.enable_grpc("https://mainnet.block-engine.jito.wtf").await?;
    
    // 使用新的 gRPC 方法
    println!("\n使用 gRPC 方法:");
    match sdk.get_tip_accounts_with_grpc().await {
        Ok(tip_accounts) => {
            println!("Tip accounts (gRPC):\n{}", serde_json::to_string_pretty(&tip_accounts)?);
        },
        Err(e) => eprintln!("gRPC Error: {:?}", e),
    }

    Ok(())
}
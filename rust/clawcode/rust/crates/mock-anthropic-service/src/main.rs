use std::env;

use mock_anthropic_service::MockAnthropicService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bind_addr = String::from("127.0.0.1:0");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => {
                bind_addr = args
                    .next()
                    .ok_or_else(|| "missing value for --bind".to_string())?;
            }
            flag if flag.starts_with("--bind=") => {
                bind_addr = flag[7..].to_string();
            }
            "--help" | "-h" => {
                println!("Usage: mock-anthropic-service [--bind HOST:PORT]");
                return Ok(());
            }
            other => {
                return Err(format!("unsupported argument: {other}").into());
            }
        }
    }

    let server = MockAnthropicService::spawn_on(&bind_addr).await?;
    println!("MOCK_ANTHROPIC_BASE_URL={}", server.base_url());
    tokio::signal::ctrl_c().await?;
    drop(server);
    Ok(())
}

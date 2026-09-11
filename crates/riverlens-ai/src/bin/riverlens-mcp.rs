#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 3 && args[1] == "--socket",
        "Usage: riverlens-mcp --socket NAME"
    );
    riverlens_ai::mcp::run(args[2].clone()).await
}

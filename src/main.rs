use clap::{Parser, Subcommand};
use futures::StreamExt;
use tracing_subscriber::EnvFilter;

use near_jsonrpc_client::{methods, JsonRpcClient};
use near_lake_framework::near_indexer_primitives::types::{BlockReference, Finality};
use near_lake_framework::LakeConfigBuilder;

#[derive(Parser, Debug, Clone)]
#[clap(version = "0.1", author = "Near Inc. <hello@nearprotocol.com>")]
pub(crate) struct Opts {
    #[clap(subcommand)]
    pub chain_id: ChainId,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ChainId {
    #[clap(subcommand)]
    Mainnet(StartOptions),
    #[clap(subcommand)]
    Testnet(StartOptions),
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum StartOptions {
    FromBlock { height: u64 },
    FromLatest,
    FromInterruption,
}

impl Opts {
    pub fn rpc_url(&self) -> &str {
        match self.chain_id {
            ChainId::Mainnet(_) => "https://rpc.mainnet.near.org",
            ChainId::Testnet(_) => "https://rpc.testnet.near.org",
        }
    }

    pub fn start_options(&self) -> &StartOptions {
        match &self.chain_id {
            ChainId::Mainnet(args) | ChainId::Testnet(args) => args
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), tokio::io::Error> {
    init_tracing();

    let opts = Opts::parse();

    let mut lake_config_builder = LakeConfigBuilder::default();

    match &opts.chain_id {
        ChainId::Mainnet(_) => {
            lake_config_builder = lake_config_builder
                .mainnet();
        }
        ChainId::Testnet(_) => {
            lake_config_builder = lake_config_builder
                .testnet();
        }
    }

    lake_config_builder = lake_config_builder
        .start_block_height(get_start_block_height(&opts).await);

    let config = lake_config_builder
        .build()
        .expect("Failed to build LakeConfig");

    let stream = near_lake_framework::streamer(config);

    let mut handlers = tokio_stream::wrappers::ReceiverStream::new(stream)
        .map(handle_streamer_message)
        .buffer_unordered(1usize);

    while let Some(_handle_message) = handlers.next().await {}

    Ok(())
}

async fn get_start_block_height(opts: &Opts) -> u64 {
    match opts.start_options() {
        StartOptions::FromBlock { height } => *height,
        StartOptions::FromLatest => final_block_height(opts.rpc_url()).await,
        StartOptions::FromInterruption => {
            match &std::fs::read("last_indexed_block") {
                Ok(contents) => {
                    String::from_utf8_lossy(contents).parse().unwrap()
                }
                Err(e) => {
                    eprintln!("Cannot read last_indexed_block.\n{}\nStart indexer from latest final", e);
                    final_block_height(opts.rpc_url()).await
                }
            }
        },
    }
}

async fn final_block_height(rpc_url: &str) -> u64 {
    let client = JsonRpcClient::connect(rpc_url);
    let request = methods::block::RpcBlockRequest {
        block_reference: BlockReference::Finality(Finality::Final),
    };

    let latest_block = client.call(request).await.unwrap();

    latest_block.header.height
}

async fn handle_streamer_message(
    streamer_message: near_lake_framework::near_indexer_primitives::StreamerMessage,
) {
    eprintln!(
        "{} / shards {}",
        streamer_message.block.header.height,
        streamer_message.shards.len()
    );
    std::fs::write("last_indexed_block", streamer_message.block.header.height.to_string().as_bytes()).unwrap();
}

pub(crate) fn init_tracing() {
    let mut env_filter = EnvFilter::new("near_lake_framework=info");

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if !rust_log.is_empty() {
            for directive in rust_log.split(',').filter_map(|s| match s.parse() {
                Ok(directive) => Some(directive),
                Err(err) => {
                    eprintln!("Ignoring directive `{}`: {}", s, err);
                    None
                }
            }) {
                env_filter = env_filter.add_directive(directive);
            }
        }
    }

    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
}

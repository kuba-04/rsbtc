use argh::FromArgs;
use dashmap::DashMap;
use static_init::dynamic;
use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use lib::types::Blockchain;
use std::path::Path;

mod util;
mod handler;

#[dynamic]
pub static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());
// node pool
#[dynamic]
pub static NODES: DashMap<String, TcpStream> = DashMap::new();


#[derive(FromArgs)]
/// A toy blockchain node
struct Args {
    #[argh(option, default="9000", description = "...")]
    port: u16,
    #[argh(
        option,
        default = "String::from(\"./blockchain.cbor\")",
        description = "..."
    )]
    blockchain_file: String,
    #[argh(positional)]
    nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let port = args.port;
    let blockchain_file = args.blockchain_file;
    let nodes = args.nodes;

    if Path::new(&blockchain_file).exists() {
        util::load_blockchain(&blockchain_file).await?;
    } else {
        println!("blockchain file does not exist!");
        util::populate_connections(&nodes).await?;
        println!("total amount of known nodes: {}", NODES.len());
        if nodes.is_empty() {
            println!("no initial nodes provided, starting as a seed node");
        } else {
            let (longest_name, longest_count) = util::find_longest_chain_node().await?;
            // request the blockchain from the node with the longest blockchain
            util::download_blockchain(
                &longest_name,
                longest_count,
            ).await?;
            println!("blockchain downloaded from {}", longest_name);
            // recalculate utxos
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.rebuild_utxos();
            }
            // try to adjust difficulty
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.try_adjust_target();
            }
        }
    }
    // Start TCP listener
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);
    // periodically cleanup the mempool
    tokio::spawn(util::cleanup());
    // periodically save the blockchain
    tokio::spawn(util::save(blockchain_file.clone()));
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handler::handle_connection(socket));
    }
}
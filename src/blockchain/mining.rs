use crate::blockchain::{Block, Consensus, MessageTransaction};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use super::Mempool;

pub enum MiningCommand {
    StartMining,
    StopMining,
    Shutdown,
}

pub struct MiningInterface<C: Consensus> {
    pub mempool_accessor: Arc<Mutex<Mempool>>,
    pub chain_info: Arc<Mutex<ChainInfo>>,
    pub consensus: C,
    pub block_channel: mpsc::Sender<(Block<C::Proof>, Vec<String>)>,
}

pub struct ChainInfo {
    pub length: u64,
    pub last_hash: String,
}

pub struct MiningCoordinator<C: Consensus> {
    command_rx: Receiver<MiningCommand>,
    mining_interface: MiningInterface<C>,
    accumulation_time_ms: u64,
    is_mining: bool,
}

impl<C: Consensus> MiningCoordinator<C>
where
    C::Proof: Serialize + Clone + Send + Sync + 'static,
{
    pub fn new(
        mining_interface: MiningInterface<C>,
        accumulation_time_ms: u64,
    ) -> (Self, Sender<MiningCommand>) {
        let (command_tx, command_rx) = mpsc::channel(32);
        (
            MiningCoordinator {
                command_rx,
                mining_interface,
                accumulation_time_ms,
                is_mining: false,
            },
            command_tx,
        )
    }

    pub async fn run(&mut self) {
        loop {
            while let Ok(command) = self.command_rx.try_recv() {
                match command {
                    MiningCommand::StartMining => {
                        println!("Start mining process");
                        self.is_mining = true;
                    }
                    MiningCommand::StopMining => {
                        println!("Stopping mining process");
                        self.is_mining = false;
                    }
                    MiningCommand::Shutdown => {
                        println!("Shutting down mining coordinator");
                        return;
                    }
                }
            }
            if self.is_mining {
                sleep(Duration::from_millis(self.accumulation_time_ms)).await;
            }

            let messages = {
                let mempool = self.mining_interface.mempool_accessor.lock().await;
                let max_messages = 10; // TODO: Parametrize this guy
                mempool.get_pending_messages(max_messages)
            };

            if !messages.is_empty() {
                if let Some((block, message_ids)) = self.mine_block(&messages).await {
                    if let Err(e) = self
                        .mining_interface
                        .block_channel
                        .send((block.clone(), message_ids))
                        .await
                    {
                        eprintln!("Error sending mined block: {}", e);
                    } else {
                        println!("Successfully minted block #{}", block.index);
                    }
                } else {
                    // No messages to mine, pause to avoid
                    // busy looping
                    sleep(Duration::from_millis(500)).await;
                }
            } else {
                // Not mining, pause to avoid busy looping
                sleep(Duration::from_millis(500)).await;
            }
        }
    }

    async fn mine_block(
        &self,
        messages: &[MessageTransaction],
    ) -> Option<(Block<C::Proof>, Vec<String>)> {
        if messages.is_empty() {
            return None;
        }

        let (chain_len, prev_hash) = {
            let chain_info = self.mining_interface.chain_info.lock().await;
            (chain_info.length, chain_info.last_hash.clone())
        };

        let data = serde_json::to_string(&messages).unwrap_or_default();
        let timestamp = chrono::Utc::now().timestamp();
        let consensus = self.mining_interface.consensus.clone();

        let proof = consensus
            .prove(chain_len, timestamp, &data, &prev_hash)
            .await;

        let block = Block::new(chain_len, data, timestamp, proof, prev_hash);

        let message_ids: Vec<String> = messages.iter().map(|tx| tx.id.clone()).collect();

        Some((block, message_ids))
    }
}

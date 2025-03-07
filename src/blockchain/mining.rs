use crate::api::client;
use crate::blockchain::{Block, Chain, Consensus, MessageTransaction};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{sleep, Duration};

pub enum MiningCommand {
    StartMining,
    StopMining,
    Shutdown,
}

pub struct MiningCoordinator<C: Consensus> {
    command_rx: Receiver<MiningCommand>,
    chain_data: Arc<Mutex<Chain<C>>>,
    accumulation_time_ms: u64,
    is_mining: bool,
}

impl<C: Consensus> MiningCoordinator<C>
where
    C::Proof: Serialize + Clone + Send + Sync + 'static,
{
    pub fn new(
        chain_data: Arc<Mutex<Chain<C>>>,
        accumulation_time_ms: u64,
    ) -> (Self, Sender<MiningCommand>) {
        let (command_tx, command_rx) = mpsc::channel(32);
        (
            MiningCoordinator {
                command_rx,
                chain_data,
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
                let chain = self.chain_data.lock().unwrap();
                let max_messages = 10; // TODO: Parametrize this guy
                chain.mempool.get_pending_messages(max_messages)
            };

            if !messages.is_empty() {
                if let Some(block) = self.mine_block(&messages).await {
                    let nodes = {
                        let chain = self.chain_data.lock().unwrap();
                        chain.nodes.clone()
                    };

                    if let Err(e) = client::broadcast_block::<C>(&nodes, &block, None).await {
                        eprintln("Error broadcasting mined block: {}", e);
                    }
                    println("Successfully mined and broadcast block #{}", block.index);
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

    async fn mine_block(&self, messages: &[MessageTransaction]) -> Option<Block<C::Proof>> {
        if messages.is_empty() {
            return None;
        }

        let data = serde_json::to_string(&messages).unwrap_or_default();
        let timestamp = chrono::Utc::now().timestamp();

        let (prev_hash, chain_len, consensus) = {
            let chain = self.chain_data.lock().unwrap();
            let prev_block = chain.chain.last().unwrap();
            (
                prev_block.hash.clone(),
                chain.chain.len() as u64,
                chain.consensus.clone(),
            )
        };

        let proof = {
            let chain = self.chain_data.lock().unwrap();
            consensus.prove(&chain, &data).await
        };

        let block = Block::new(chain_len, data, timestamp, proof, prev_hash);
        {
            let mut chain = self.chain_data.lock().unwrap();
            if chain.chain.len() as u64 == chain_len {
                let message_ids: Vec<String> = messages.iter().map(|tx| tx.id.clone()).collect();
                chain.mempool.remove_messages(&message_ids);

                chain.chain.push(block.clone());
                Some(block)
            } else {
                println!("Chain changes during mining, discarding block");
                None
            }
        }
    }
}

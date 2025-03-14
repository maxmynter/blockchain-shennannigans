use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, oneshot};
use uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTransaction {
    pub id: String,
    pub message: String,
    pub timestamp: i64,
    pub submitted_at: i64,
}

impl MessageTransaction {
    pub fn new(message: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        Self {
            id,
            message,
            timestamp: now,
            submitted_at: now,
        }
    }
}

#[derive(Clone)]
pub struct MessageQueue {
    sender: mpsc::Sender<(String, oneshot::Sender<()>)>,
}

impl MessageQueue {
    pub fn new(mempool: Arc<TokioMutex<Mempool>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<(String, oneshot::Sender<()>)>(100);

        tokio::spawn(async move {
            while let Some((message, confirmation)) = rx.recv().await {
                let mut pool = mempool.lock().await;
                let _ = pool.add_message(message);
                let _ = confirmation.send(());
            }
        });
        MessageQueue { sender: tx }
    }

    pub async fn submit_message(&self, message: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        match self.sender.send((message, tx)).await {
            Ok(_) => {
                rx.await
                    .map_err(|_| "Failed to receive queuing confirmation".to_string())?;
                Ok(())
            }
            Err(_) => Err("Failed to send queue message".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mempool {
    pending_messages: HashMap<String, MessageTransaction>,
    #[serde(skip)]
    message_added_at: HashMap<String, Instant>,
    max_size: usize,
    #[serde(skip)]
    message_timeout: Duration,
}

impl Mempool {
    pub fn new(max_size: usize, timeout_secs: u64) -> Self {
        Self {
            pending_messages: HashMap::new(),
            message_added_at: HashMap::new(),
            max_size,
            message_timeout: Duration::from_secs(timeout_secs),
        }
    }
    pub fn add_message(&mut self, message: String) -> Result<MessageTransaction, String> {
        if self.pending_messages.len() > self.max_size {
            self.clean_expired_messages();
            if self.pending_messages.len() > self.max_size {
                return Err("Mempool is full".to_string());
            }
        }
        let transaction = MessageTransaction::new(message);
        self.pending_messages
            .insert(transaction.id.clone(), transaction.clone());
        self.message_added_at
            .insert(transaction.id.clone(), Instant::now());
        Ok(transaction)
    }

    pub fn get_pending_messages(&self, limit: usize) -> Vec<MessageTransaction> {
        self.pending_messages
            .values()
            .cloned()
            .take(limit)
            .collect()
    }

    pub fn remove_messages(&mut self, ids: &[String]) {
        for id in ids {
            self.pending_messages.remove(id);
            self.message_added_at.remove(id);
        }
        println!(
            "Removed {} messages from mempool. {} remaining",
            ids.len(),
            self.pending_count()
        );
    }

    pub fn clean_expired_messages(&mut self) {
        let now = Instant::now();
        let expired_ids: Vec<String> = self
            .message_added_at
            .iter()
            .filter(|(_, added_at)| now.duration_since(**added_at) > self.message_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            self.pending_messages.remove(&id);
            self.message_added_at.remove(&id);
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_messages.len()
    }
}

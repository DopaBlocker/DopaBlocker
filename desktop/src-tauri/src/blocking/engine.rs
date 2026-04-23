// =============================================================================
// Engine — orquestrador do bloqueio.
// =============================================================================
// Nesta etapa só controla o DNS proxy. WFP (filtro kernel-level) e a troca
// do DNS do sistema entram na etapa seguinte.
//
// Contrato do engine:
//   - `start(rules)` sobe o DNS proxy em background, guarda o handle e o
//     canal de shutdown para poder parar depois. Idempotente: chamar com
//     engine já rodando retorna `AlreadyRunning`.
//   - `stop()` dispara shutdown, aguarda a task encerrar. Idempotente.
//   - `update_rules(rules)` troca a blocklist a quente, sem derrubar o proxy.
//   - `is_running()` reflete o estado interno.
// =============================================================================

use std::{collections::HashSet, sync::Arc};

use tokio::{
    sync::{oneshot, RwLock},
    task::JoinHandle,
};

use super::dns_proxy;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("engine já está rodando")]
    AlreadyRunning,
    #[error("DNS proxy: {0}")]
    Dns(#[from] anyhow::Error),
}

pub struct Engine {
    rules: Arc<RwLock<HashSet<String>>>,
    task: Option<JoinHandle<()>>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashSet::new())),
            task: None,
            shutdown: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.task.is_some()
    }

    pub async fn start(&mut self, initial_rules: Vec<String>) -> Result<(), EngineError> {
        if self.is_running() {
            return Err(EngineError::AlreadyRunning);
        }

        {
            let mut w = self.rules.write().await;
            w.clear();
            w.extend(initial_rules);
        }

        let (tx, rx) = oneshot::channel();
        let rules = self.rules.clone();

        // A task pode falhar se a porta 53 não puder ser bindada. Nesse caso
        // logamos e o `is_running()` continuará verdade até o próximo stop —
        // mas um bind falho sai rápido, então o usuário verá no log e pode
        // tentar de novo.
        let task = tokio::spawn(async move {
            match dns_proxy::run(rules, rx).await {
                Ok(()) => tracing::info!("DNS proxy encerrado normalmente"),
                Err(e) => tracing::error!(error = %e, "DNS proxy morreu"),
            }
        });

        self.task = Some(task);
        self.shutdown = Some(tx);
        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            // Ignora erro: se o receiver já foi dropado, a task já saiu.
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            // Aguarda a task limpar socket. Se panicar, só logamos.
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do DNS proxy terminou com erro");
            }
        }
    }

    pub async fn update_rules(&self, new_rules: Vec<String>) {
        let mut w = self.rules.write().await;
        w.clear();
        w.extend(new_rules);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

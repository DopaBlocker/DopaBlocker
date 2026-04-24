// =============================================================================
// Engine — orquestrador do bloqueio.
// =============================================================================
// Três camadas, por ordem de aparecer no fluxo de uma request bloqueada:
//   1. DNS proxy: vê a query DNS e devolve A=127.0.0.1 se bloqueada.
//   2. Block page HTTP: serve a página "Este site está bloqueado" em
//      127.0.0.1:80 — o browser conecta lá pra HTTP bloqueado.
//   3. WFP (Windows-only): filtros kernel-level que impedem bypass via
//      DoH/DoT/IP-direto.
//
// Cada camada é opcional — se uma falhar ao subir (sem admin, porta ocupada),
// as outras continuam rodando com funcionalidade degradada.
// =============================================================================

use std::{collections::HashSet, sync::Arc};

use tokio::{
    sync::{oneshot, RwLock},
    task::JoinHandle,
};

use super::{adult_filter::AdultFilter, block_page, dns_proxy};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("engine já está rodando")]
    AlreadyRunning,
    #[error("DNS proxy: {0}")]
    Dns(#[from] anyhow::Error),
}

pub struct Engine {
    rules: Arc<RwLock<HashSet<String>>>,
    adult_filter: Arc<AdultFilter>,
    dns_task: Option<JoinHandle<()>>,
    dns_shutdown: Option<oneshot::Sender<()>>,
    block_page_task: Option<JoinHandle<()>>,
    block_page_shutdown: Option<oneshot::Sender<()>>,
    #[cfg(target_os = "windows")]
    wfp: Option<super::wfp::WfpSession>,
}

impl Engine {
    pub fn new(adult_filter: Arc<AdultFilter>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashSet::new())),
            adult_filter,
            dns_task: None,
            dns_shutdown: None,
            block_page_task: None,
            block_page_shutdown: None,
            #[cfg(target_os = "windows")]
            wfp: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.dns_task.is_some()
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

        // ---- Block page HTTP -------------------------------------------------
        // Sobe antes do DNS proxy: se o DNS já começou a devolver
        // A=127.0.0.1 antes do HTTP estar pronto, user veria "connection
        // refused" por alguns ms. Iniciar aqui primeiro minimiza a janela.
        let (bp_tx, bp_rx) = oneshot::channel();
        let block_page_task = tokio::spawn(async move {
            match block_page::run(bp_rx).await {
                Ok(()) => tracing::info!("block page encerrou normalmente"),
                Err(e) => {
                    tracing::warn!(error = %e, "block page falhou — HTTP bloqueado vai ficar sem UI")
                }
            }
        });
        self.block_page_task = Some(block_page_task);
        self.block_page_shutdown = Some(bp_tx);

        // ---- DNS proxy -------------------------------------------------------
        let (dns_tx, dns_rx) = oneshot::channel();
        let rules = self.rules.clone();
        let adult = self.adult_filter.clone();
        let dns_task = tokio::spawn(async move {
            match dns_proxy::run(rules, adult, dns_rx).await {
                Ok(()) => tracing::info!("DNS proxy encerrado normalmente"),
                Err(e) => tracing::error!(error = %e, "DNS proxy morreu"),
            }
        });
        self.dns_task = Some(dns_task);
        self.dns_shutdown = Some(dns_tx);

        // ---- WFP (Windows) ---------------------------------------------------
        // Depois do DNS proxy: os filtros "DNS ≠ 127.0.0.1 → BLOCK" só fazem
        // sentido se tem alguém em 127.0.0.1 pra servir a query.
        #[cfg(target_os = "windows")]
        {
            match super::wfp::WfpSession::install() {
                Ok(session) => self.wfp = Some(session),
                Err(e) => {
                    tracing::warn!(error = %e, "WFP não instalado — bloqueio fica só no DNS proxy");
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        // Ordem inversa do start: primeiro WFP (deixa tráfego fluir pro DNS
        // original caso ainda esteja apontando pra cá), depois DNS proxy,
        // por último o block page.
        #[cfg(target_os = "windows")]
        {
            self.wfp.take(); // Drop dispara FwpmEngineClose0
        }

        if let Some(tx) = self.dns_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.dns_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do DNS proxy terminou com erro");
            }
        }

        if let Some(tx) = self.block_page_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.block_page_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do block page terminou com erro");
            }
        }
    }

    pub async fn update_rules(&self, new_rules: Vec<String>) {
        let mut w = self.rules.write().await;
        w.clear();
        w.extend(new_rules);
    }
}

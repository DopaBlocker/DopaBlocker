// =============================================================================
// Engine: orchestrates the local blocking stack.
// =============================================================================
// Start order matters:
//   1. Load/install the local CA used by the HTTPS block page.
//   2. Start block page HTTP (:80).
//   3. Start block page HTTPS (:443).
//   4. Start DNS proxy (:53), which redirects blocked domains to 127.0.0.1.
//   5. Install WFP filters on Windows.
//
// Each visible page layer is best-effort. If :443 is busy or the CA cannot be
// installed, DNS blocking still works; the browser may just show its own error.
// =============================================================================

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use tokio::{
    sync::{oneshot, RwLock},
    task::JoinHandle,
};

use super::{adult_filter::AdultFilter, block_page, ca::LocalCa, dns_proxy};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("engine ja esta rodando")]
    AlreadyRunning,
    #[error("DNS proxy: {0}")]
    Dns(#[from] anyhow::Error),
}

pub struct Engine {
    rules: Arc<RwLock<HashSet<String>>>,
    adult_filter: Arc<AdultFilter>,
    app_data_dir: PathBuf,
    dns_task: Option<JoinHandle<()>>,
    dns_shutdown: Option<oneshot::Sender<()>>,
    block_page_http_task: Option<JoinHandle<()>>,
    block_page_http_shutdown: Option<oneshot::Sender<()>>,
    block_page_https_task: Option<JoinHandle<()>>,
    block_page_https_shutdown: Option<oneshot::Sender<()>>,
    #[cfg(target_os = "windows")]
    wfp: Option<super::wfp::WfpSession>,
}

impl Engine {
    pub fn new(adult_filter: Arc<AdultFilter>, app_data_dir: PathBuf) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashSet::new())),
            adult_filter,
            app_data_dir,
            dns_task: None,
            dns_shutdown: None,
            block_page_http_task: None,
            block_page_http_shutdown: None,
            block_page_https_task: None,
            block_page_https_shutdown: None,
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

        let ca = match LocalCa::load_or_create(&self.app_data_dir) {
            Ok(ca) => {
                let ca = Arc::new(ca);
                let status = ca.install_in_windows_root();
                tracing::info!(
                    status = ?status,
                    thumbprint = %ca.thumbprint(),
                    "resultado da instalacao da CA local",
                );
                Some(ca)
            }
            Err(e) => {
                tracing::warn!(error = %e, "CA local indisponivel; HTTPS block page nao vai subir");
                None
            }
        };

        let (bp_http_tx, bp_http_rx) = oneshot::channel();
        let rules_for_http = self.rules.clone();
        let adult_for_http = self.adult_filter.clone();
        let block_page_http_task = tokio::spawn(async move {
            match block_page::run_http(rules_for_http, adult_for_http, bp_http_rx).await {
                Ok(()) => tracing::info!("block page HTTP encerrou normalmente"),
                Err(e) => {
                    tracing::warn!(error = %e, "block page HTTP falhou; HTTP bloqueado vai ficar sem UI")
                }
            }
        });
        self.block_page_http_task = Some(block_page_http_task);
        self.block_page_http_shutdown = Some(bp_http_tx);

        if let Some(ca) = ca {
            let (bp_https_tx, bp_https_rx) = oneshot::channel();
            let rules_for_https = self.rules.clone();
            let adult_for_https = self.adult_filter.clone();
            let block_page_https_task = tokio::spawn(async move {
                match block_page::run_https(rules_for_https, adult_for_https, ca, bp_https_rx).await
                {
                    Ok(()) => tracing::info!("block page HTTPS encerrou normalmente"),
                    Err(e) => {
                        tracing::warn!(error = %e, "block page HTTPS falhou; sites HTTPS bloqueados podem mostrar erro do browser")
                    }
                }
            });
            self.block_page_https_task = Some(block_page_https_task);
            self.block_page_https_shutdown = Some(bp_https_tx);
        }

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

        #[cfg(target_os = "windows")]
        {
            match super::wfp::WfpSession::install() {
                Ok(session) => self.wfp = Some(session),
                Err(e) => {
                    tracing::warn!(error = %e, "WFP nao instalado; bloqueio fica so no DNS proxy");
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            self.wfp.take();
        }

        if let Some(tx) = self.dns_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.dns_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do DNS proxy terminou com erro");
            }
        }

        if let Some(tx) = self.block_page_https_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.block_page_https_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do block page HTTPS terminou com erro");
            }
        }

        if let Some(tx) = self.block_page_http_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.block_page_http_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = %e, "task do block page HTTP terminou com erro");
            }
        }
    }

    pub async fn update_rules(&self, new_rules: Vec<String>) {
        let mut w = self.rules.write().await;
        w.clear();
        w.extend(new_rules);
    }
}

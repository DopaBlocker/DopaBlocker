// =============================================================================
// Lifecycle — dono único da orquestração do bloqueio.
// =============================================================================
// O `Engine` cuida só do stack in-process (WFP, CA, páginas, DNS proxy). A
// troca do DNS do sistema (`os::system_dns`) e a persistência do flag no DB
// vivem aqui, coordenadas numa única sequência por operação. Antes essa mesma
// coreografia ("engine + DNS + rollback + flag") estava DUPLICADA em
// `commands.rs` (set_blocking_enabled), e em `lib.rs` (resume e shutdown).
//
// Os handlers `#[tauri::command]` (commands.rs) e o ciclo de vida do app
// (lib.rs) passam a só CHAMAR estas funções.
//
// IMPORTANTE: o restore SÍNCRONO de DNS (panic hook / SetConsoleCtrlHandler /
// RunEvent) NÃO mora aqui — fica em `os::system_dns::restore_dns_blocking_*`,
// sem tokio/SQLCipher, porque precisa rodar de contextos que não podem usar
// async nem reabrir o banco.
// =============================================================================

use std::path::Path;

use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

use dopablocker_shared::models::BlockMode;
use dopablocker_shared::parental::{effective_strategy, BlocklistStrategy};

use crate::blocking::engine::Engine;
use crate::blocking::os::system_dns;
use crate::db;

/// Contexto da regra do "pai imune" que o frontend envia em cada operação do
/// engine. Sem isto, o engine não tem como distinguir um device pai (que deve
/// ficar imune) de um pessoal (que aplica tudo) — a info não vive no SQLCipher
/// local, mora no auth store do frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct ParentalContext {
    pub mode: BlockMode,
    pub is_child: bool,
}

impl ParentalContext {
    pub(crate) fn personal() -> Self {
        Self {
            mode: BlockMode::Personal,
            is_child: false,
        }
    }
}

/// Regras efetivas para o device, aplicando a regra do pai imune: pai em modo
/// parental recebe lista vazia (engine roda mas não bloqueia nada); device
/// pessoal e device filho recebem a lista cheia do dono.
async fn effective_rules(
    conn: &Connection,
    user_id: String,
    ctx: &ParentalContext,
) -> Result<Vec<String>, String> {
    match effective_strategy(ctx.mode.clone(), ctx.is_child) {
        BlocklistStrategy::Empty => Ok(Vec::new()),
        BlocklistStrategy::ApplyAll => db::list_active_domains(conn, user_id)
            .await
            .map_err(|e| e.to_string()),
    }
}

/// Recarrega as regras ativas do DB para o engine — chamar sempre que o cache
/// local muda e o engine estiver rodando. Se não estiver, no-op.
pub(crate) async fn refresh_engine_rules(
    conn: &Connection,
    engine: &Mutex<Engine>,
    user_id: String,
    ctx: &ParentalContext,
) -> Result<(), String> {
    let eng = engine.lock().await;
    if !eng.is_running() {
        return Ok(());
    }
    let rules = effective_rules(conn, user_id, ctx).await?;
    eng.update_rules(rules).await;
    Ok(())
}

/// Liga o bloqueio: sobe o proxy, aponta o DNS do sistema para ele e persiste
/// o flag. Idempotente — se o engine já roda, só re-aplica as regras.
pub async fn enable(
    engine: &Mutex<Engine>,
    conn: &Connection,
    data_dir: &Path,
    user_id: String,
    ctx: &ParentalContext,
) -> anyhow::Result<()> {
    // 1. Sobe o proxy. Se falhar no bind (porta 53 sem admin, já ocupada),
    //    nem chegamos a mexer no DNS do sistema.
    //
    // Aplica a regra do pai imune: device do pai em modo parental recebe lista
    // vazia (engine roda mas nao bloqueia nada). Device pessoal e device filho
    // recebem a lista cheia.
    let rules = effective_rules(conn, user_id.clone(), ctx)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let mut eng = engine.lock().await;
        if eng.is_running() {
            eng.update_rules(rules).await;
        } else {
            eng.start(rules).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }

    // 2. Aponta o DNS do sistema pro proxy. Se falhar (tipicamente admin),
    //    rollback no engine — melhor desligado do que meio-configurado.
    if let Err(e) = system_dns::apply_and_remember(conn, data_dir).await {
        let mut eng = engine.lock().await;
        eng.stop().await;
        anyhow::bail!("falha ao trocar DNS do sistema: {e}");
    }

    db::set_blocking_enabled(conn, true).await?;
    db::set_last_active_user_id(conn, user_id).await?;
    Ok(())
}

/// Desliga o bloqueio: restaura o DNS do sistema ANTES de matar o proxy (senão
/// haveria uma janela em que o sistema aponta pra 127.0.0.1:53 sem ninguém
/// escutando → DNS quebrado), para o engine e persiste o flag.
pub async fn disable(
    engine: &Mutex<Engine>,
    conn: &Connection,
    data_dir: &Path,
    user_id: String,
) -> anyhow::Result<()> {
    if let Err(e) = system_dns::restore_if_any(conn, data_dir).await {
        tracing::error!(error = %e, "falha ao restaurar DNS — seguindo pra parar engine");
    }
    {
        let mut eng = engine.lock().await;
        eng.stop().await;
    }

    db::set_blocking_enabled(conn, false).await?;
    db::set_last_active_user_id(conn, user_id).await?;
    Ok(())
}

/// Reativa o engine no boot se `blocking_enabled` estava persistido. Faz o
/// `restore_if_any` (o `heal_orphan_dns` já rodou síncrono no setup), valida o
/// DNS atual e só então sobe o engine + reaplica o DNS do sistema.
///
/// NOTA: ao contrário de `enable`, o resume carrega `list_active_domains`
/// **sem** o filtro de pai imune — o `ParentalContext` vive no frontend e não
/// é persistido. Comportamento mantido de propósito.
pub async fn resume_if_enabled(
    engine: &Mutex<Engine>,
    conn: &Connection,
    data_dir: &Path,
) -> anyhow::Result<()> {
    // Passo 1: se o restore falhou, NÃO reativamos o engine — senão o
    // `apply_and_remember` faria `capture_current` do estado quebrado,
    // persistindo loopback como "DNS original" (bug raiz).
    let restore_failed = match system_dns::restore_if_any(conn, data_dir).await {
        Ok(()) => false,
        Err(e) => {
            tracing::error!(error = %e, "falha ao restaurar DNS no boot — desativando bloqueio por seguranca");
            true
        }
    };
    if restore_failed {
        let _ = db::set_blocking_enabled(conn, false).await;
        return Ok(());
    }

    let current_dns = match system_dns::capture_current().await {
        Ok(snapshots) if !snapshots.is_empty() => snapshots,
        Ok(_) => {
            tracing::error!(
                "nenhuma interface DNS elegivel no boot; bloqueio desativado por seguranca"
            );
            let _ = db::set_blocking_enabled(conn, false).await;
            return Ok(());
        }
        Err(e) => {
            tracing::error!(error = %e, "falha ao capturar DNS no boot; bloqueio desativado por seguranca");
            let _ = db::set_blocking_enabled(conn, false).await;
            return Ok(());
        }
    };
    tracing::debug!(
        interfaces = current_dns.len(),
        "preflight de DNS aprovado para resume"
    );

    if !db::get_blocking_enabled(conn).await? {
        return Ok(());
    }
    let Some(user_id) = db::get_last_active_user_id(conn).await? else {
        tracing::info!("flag blocking_enabled=true mas sem last_active_user_id — pulando resume");
        return Ok(());
    };
    let rules = db::list_active_domains(conn, user_id.clone()).await?;

    {
        let mut eng = engine.lock().await;
        if eng.is_running() {
            return Ok(());
        }
        eng.start(rules).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Engine de pé — agora captura o DNS atual (já restaurado acima, então é o
    // "de verdade") e aponta pro proxy. Se falhar, desliga tudo pra não deixar
    // bloqueio meio-ativo.
    if let Err(e) = system_dns::apply_and_remember(conn, data_dir).await {
        tracing::warn!(error = %e, "falha ao reaplicar DNS no resume — revertendo");
        let mut eng = engine.lock().await;
        eng.stop().await;
        return Err(e);
    }

    tracing::info!(%user_id, "engine reativado no boot");
    Ok(())
}

/// Saída pelo tray ("Sair e desligar bloqueio"): restaura o DNS, para o engine
/// e persiste o flag desligado. Best-effort — loga erros mas não aborta, pois
/// o objetivo final é sair limpo.
pub async fn shutdown_and_disable(engine: &Mutex<Engine>, conn: &Connection, data_dir: &Path) {
    if let Err(e) = system_dns::restore_if_any(conn, data_dir).await {
        tracing::error!(error = %e, "falha ao restaurar DNS ao sair pelo tray");
    }

    {
        let mut eng = engine.lock().await;
        eng.stop().await;
    }

    if let Err(e) = db::set_blocking_enabled(conn, false).await {
        tracing::error!(error = %e, "falha ao persistir bloqueio desligado ao sair");
    }
}

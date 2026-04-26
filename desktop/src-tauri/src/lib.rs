// =============================================================================
// Boot do app Tauri.
// =============================================================================
// No setup:
//   1. Registra o plugin de log em debug.
//   2. Inicializa o SQLCipher local (db::init) e registra a Connection
//      como state — qualquer comando pega via `State<'_, Connection>`.
//   3. Cria o AdultFilter com o estado persistido e dispara o build em
//      background (download+populate). O DNS proxy trata `None` como "não
//      construído ainda" então o boot da janela nunca é bloqueado.
//   4. Cria o Engine (segurando Arc do AdultFilter) em estado parado e
//      registra como state (Arc<Mutex<_>> pra permitir o resume assíncrono).
//   5. Se `blocking_enabled=true` estava persistido, spawna uma task que
//      restaura DNS órfão do crash anterior (se houver), reativa o engine
//      e aplica DNS do sistema.
// =============================================================================

mod blocking;
mod commands;
mod db;

use std::{path::PathBuf, sync::Arc};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};
use tokio::sync::Mutex;

use blocking::{adult_filter::AdultFilter, engine::Engine};

#[derive(Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Boot síncrono do DB — só roda uma vez, antes da janela abrir.
            let handle = app.handle().clone();
            let conn = tauri::async_runtime::block_on(async move { db::init(&handle).await })?;

            // Adult filter: carrega o estado persistido do DB (ligado/desligado)
            // e cria a instância. O Bloom Filter propriamente dito é construído
            // em background — enquanto isso `contains()` devolve false, então
            // DNS queries nunca esperam o download.
            let cache_dir = app
                .path()
                .app_cache_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            let persisted_adult_enabled = tauri::async_runtime::block_on(async {
                db::get_adult_filter_enabled(&conn).await.unwrap_or(false)
            });
            let adult_filter = Arc::new(AdultFilter::new(cache_dir, persisted_adult_enabled));

            // Build assíncrono — não bloqueia abertura da janela.
            let adult_for_build = adult_filter.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = adult_for_build.build_if_needed().await {
                    tracing::warn!(error = %e, "adult filter: build falhou");
                }
            });

            let engine = Arc::new(Mutex::new(Engine::new(
                adult_filter.clone(),
                data_dir.clone(),
            )));

            // Resume do engine em background. Não bloqueia o setup — se falhar
            // (sem admin pra porta 53, por ex.), o usuário vê a UI com estado
            // correto e pode tentar de novo.
            let conn_resume = conn.clone();
            let engine_resume = engine.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = resume_engine_if_enabled(&conn_resume, &engine_resume).await {
                    tracing::warn!(error = %e, "não foi possível reativar engine no boot");
                }
            });

            app.manage(conn);
            app.manage(engine);
            app.manage(adult_filter);
            app.manage(AppPaths { data_dir });

            install_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let engine = window
                    .app_handle()
                    .state::<Arc<Mutex<Engine>>>()
                    .inner()
                    .clone();
                let is_running =
                    tauri::async_runtime::block_on(async { engine.lock().await.is_running() });

                if is_running {
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        tracing::warn!(error = %e, "falha ao esconder janela no fechamento");
                    } else {
                        tracing::info!("janela escondida; bloqueio segue ativo em background");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::list_cached_blocklist,
            commands::save_blocklist,
            commands::cache_add_item,
            commands::cache_remove_item,
            commands::set_blocking_enabled,
            commands::set_adult_filter_enabled,
            commands::get_blocking_status,
            commands::install_ca_root,
            commands::save_child_session,
            commands::load_child_session,
            commands::clear_child_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn install_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let quit_disable = MenuItem::with_id(
        app,
        "quit_disable",
        "Sair e desligar bloqueio",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&open, &quit_disable])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("DopaBlocker")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "quit_disable" => shutdown_blocking_and_exit(app.clone()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main_window(tray.app_handle()),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!("janela principal nao encontrada ao abrir pelo tray");
        return;
    };

    if let Err(e) = window.show() {
        tracing::warn!(error = %e, "falha ao mostrar janela pelo tray");
    }
    if let Err(e) = window.set_focus() {
        tracing::warn!(error = %e, "falha ao focar janela pelo tray");
    }
}

fn shutdown_blocking_and_exit(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let conn = app.state::<tokio_rusqlite::Connection>().inner().clone();
        let engine = app.state::<Arc<Mutex<Engine>>>().inner().clone();

        if let Err(e) = blocking::system_dns::restore_if_any(&conn).await {
            tracing::error!(error = %e, "falha ao restaurar DNS ao sair pelo tray");
        }

        {
            let mut eng = engine.lock().await;
            eng.stop().await;
        }

        if let Err(e) = db::set_blocking_enabled(&conn, false).await {
            tracing::error!(error = %e, "falha ao persistir bloqueio desligado ao sair");
        }

        app.exit(0);
    });
}

async fn resume_engine_if_enabled(
    conn: &tokio_rusqlite::Connection,
    engine: &Mutex<Engine>,
) -> anyhow::Result<()> {
    // Passo 0: self-heal independente de snapshot. Se alguma interface
    // ainda esta apontando pra 127.0.0.1 (zumbi), forcamos DHCP. Roda
    // ANTES de qualquer outra coisa — garante que mesmo um snapshot
    // perdido no DB ou um restore que falha nao impede self-recovery.
    if let Err(e) = blocking::system_dns::heal_orphan_dns().await {
        tracing::error!(error = %e, "self-heal de DNS orfao falhou");
    }

    // Passo 1: tenta o restore baseado no snapshot persistido — se houver.
    // Cenário: app crashou com bloqueio ativo → DNS do sistema ainda está
    // apontando pro proxy que morreu → o OS fica sem resolver. Aqui,
    // colocamos DNS de volta no que era, limpamos a snapshot, e só depois
    // decidimos se reativamos o engine (que fará uma captura fresca).
    let restore_failed = match blocking::system_dns::restore_if_any(conn).await {
        Ok(()) => false,
        Err(e) => {
            tracing::error!(error = %e, "falha ao restaurar DNS no boot — desativando bloqueio por seguranca");
            true
        }
    };

    // Passo 2 (resume conservador): se o restore falhou, NAO reativamos o
    // engine — caso contrario, `apply_and_remember` faria `capture_current`
    // do estado quebrado, persistindo loopback como "DNS original" (bug raiz).
    if restore_failed {
        let _ = db::set_blocking_enabled(conn, false).await;
        return Ok(());
    }

    let current_dns = match blocking::system_dns::capture_current().await {
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

    // Engine de pé — agora captura o DNS atual (já restaurado acima, então
    // é o "de verdade") e aponta pro proxy. Se falhar, desliga tudo pra não
    // deixar bloqueio meio-ativo.
    if let Err(e) = blocking::system_dns::apply_and_remember(conn).await {
        tracing::warn!(error = %e, "falha ao reaplicar DNS no resume — revertendo");
        let mut eng = engine.lock().await;
        eng.stop().await;
        return Err(e);
    }

    tracing::info!(%user_id, "engine reativado no boot");
    Ok(())
}

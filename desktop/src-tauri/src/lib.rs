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
//
// Cleanup-de-DNS em saidas nao-graceful:
//   - panic::set_hook -> restore sincrono via snapshot file
//   - RunEvent::ExitRequested -> idem (cobre shutdown/logoff do Windows)
//   - SetConsoleCtrlHandler (Windows) -> idem (ultima trincheira)
//   - heal_orphan_dns SINCRONO no setup -> recovery se o app caiu antes
//
// Ver `blocking::os::system_dns::restore_dns_blocking_global` para detalhes.
// =============================================================================

mod blocking;
mod commands;
mod db;

use std::{path::PathBuf, sync::Arc};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, RunEvent, WindowEvent,
};
use tokio::sync::Mutex;

use blocking::{engine::Engine, policy::adult_filter::AdultFilter};

#[derive(Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Resolve data_dir cedo — usado pelo logging persistido, snapshot DNS
            // e por tudo que precise do app data dir.
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            std::fs::create_dir_all(&data_dir).ok();

            // Logging persistido em arquivo. Sem isso, debugar bug em producao
            // (ex: panic durante shutdown) e impossivel — stderr some quando o
            // processo morre. Rotacao diaria automatica.
            let logs_dir = data_dir.join("logs");
            std::fs::create_dir_all(&logs_dir).ok();
            let file_appender = tracing_appender::rolling::daily(&logs_dir, "dopablocker.log");
            // Guard precisa viver enquanto o app rodar — vazamos via Box::leak
            // para nao precisar carregar pelo state. Aceitavel: 1 instancia.
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            Box::leak(Box::new(guard));
            // Subscriber best-effort — se outro ja esta global (caso testes),
            // try_init falha silenciosamente.
            let _ = tracing_subscriber::fmt()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .try_init();

            // Inicializa SNAPSHOT_DIR para o restore sincrono. Tem que rodar
            // antes do panic hook (caso contrario o hook nao acha o snapshot).
            blocking::os::system_dns::init_snapshot_dir(data_dir.clone());

            // Panic hook — qualquer panic durante o ciclo de vida do app
            // restaura o DNS antes de propagar. Em release o `panic = unwind`
            // (default Rust) garante execucao do hook.
            install_panic_hook();

            // Windows-only: SetConsoleCtrlHandler para CTRL_SHUTDOWN_EVENT etc.
            #[cfg(target_os = "windows")]
            install_ctrl_handler();

            // Boot síncrono do DB — só roda uma vez, antes da janela abrir.
            let handle = app.handle().clone();
            let conn = tauri::async_runtime::block_on(async move { db::init(&handle).await })?;

            // Self-heal SINCRONO antes de qualquer outra coisa: se o app caiu
            // antes com DNS apontando para 127.0.0.1, ja conserta agora — antes
            // mesmo da janela aparecer. Depois disso o usuario nao ve "rede
            // caida" enquanto o setup completa.
            tauri::async_runtime::block_on(async {
                if let Err(e) = blocking::os::system_dns::heal_orphan_dns().await {
                    tracing::error!(error = %e, "self-heal sincrono de DNS orfao falhou");
                }
            });

            // Adult filter: carrega o estado persistido do DB (ligado/desligado)
            // e cria a instância. O Bloom Filter propriamente dito é construído
            // em background — enquanto isso `contains()` devolve false, então
            // DNS queries nunca esperam o download.
            let cache_dir = app
                .path()
                .app_cache_dir()
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
            let data_dir_resume = data_dir.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = blocking::lifecycle::resume_if_enabled(
                    &engine_resume,
                    &conn_resume,
                    &data_dir_resume,
                )
                .await
                {
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Captura RunEvent::ExitRequested (e Exit) para garantir que o DNS volta
    // ao normal mesmo em shutdown/logoff do Windows. Como o Tauri ja chamou
    // `setup`, `init_snapshot_dir` foi chamado — `restore_dns_blocking_global`
    // tem o data_dir.
    app.run(|_app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            blocking::os::system_dns::restore_dns_blocking_global();
        }
    });
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Restaura DNS PRIMEIRO. Se o panic for durante o boot (antes de
        // SNAPSHOT_DIR ser inicializado), e no-op silencioso — nada de DNS
        // foi alterado ainda.
        blocking::os::system_dns::restore_dns_blocking_global();
        // Encadeia o hook anterior para preservar o stack trace padrao.
        prev(info);
    }));
}

#[cfg(target_os = "windows")]
fn install_ctrl_handler() {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Console::{
        SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT,
        CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    };

    unsafe extern "system" fn handler(ctrl_type: u32) -> BOOL {
        // Em qualquer evento de saida, restaura o DNS. Retorna FALSE para
        // deixar o handler default tambem rodar — ele eventualmente termina
        // o processo.
        match ctrl_type {
            CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
            | CTRL_SHUTDOWN_EVENT => {
                blocking::os::system_dns::restore_dns_blocking_global();
                BOOL(0) // FALSE — chain to default
            }
            _ => BOOL(0),
        }
    }

    unsafe {
        if SetConsoleCtrlHandler(Some(handler), true).is_err() {
            tracing::warn!("SetConsoleCtrlHandler falhou; cleanup em shutdown pode nao rodar");
        }
    }
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
        let paths = app.state::<AppPaths>().inner().clone();

        blocking::lifecycle::shutdown_and_disable(&engine, &conn, &paths.data_dir).await;

        app.exit(0);
    });
}

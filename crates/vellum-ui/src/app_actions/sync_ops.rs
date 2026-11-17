//! Gestionnaires de synchronisation cloud, liseuses connectées et envoi SMTP/Kindle.

use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::{translate, translate_fmt, FolderSyncBackend, SecretStore, SyncService};
use vellum_domain::{Book, CloudSyncMode, DeviceKind, SmtpConfig};

use super::{show_toast, AppActionContext};

pub(super) struct SyncAndDeviceHandlers {
    pub on_export_to_device: EventHandler<(String, DeviceKind)>,
    pub on_sync_device: EventHandler<DeviceKind>,
    pub on_sync_now: EventHandler<()>,
    pub on_pick_sync_folder: EventHandler<()>,
    pub on_auto_detect_folder: EventHandler<()>,
    pub on_toggle_auto_download: EventHandler<bool>,
    pub on_toggle_sync_enabled: EventHandler<bool>,
    pub on_set_sync_mode: EventHandler<CloudSyncMode>,
    pub on_save_gdrive_creds: EventHandler<(String, String, String)>,
    pub on_connect_gdrive: EventHandler<()>,
    pub on_disconnect_gdrive: EventHandler<()>,
    pub on_save_smtp: EventHandler<SmtpConfig>,
    pub on_test_smtp: EventHandler<SmtpConfig>,
    pub on_send_to_kindle: EventHandler<Book>,
}

fn persist_sync_config(
    ss: &SyncService,
    bs: &BookService,
    mut sync_config: Signal<vellum_domain::CloudSyncConfig>,
) {
    let cfg = ss.config();
    let secrets = SecretStore::default();
    match cfg.google_client_secret.as_deref() {
        Some(secret) if !secret.trim().is_empty() => {
            let _ = secrets.set_secret("gdrive_client_secret", secret);
        }
        _ => {
            let _ = secrets.delete_secret("gdrive_client_secret");
        }
    }
    let mut persisted_cfg = cfg.clone();
    persisted_cfg.google_client_secret = None;
    if let Ok(cfg_json) = serde_json::to_string(&persisted_cfg) {
        let _ = bs.param_repo().set_value("cloud_sync_config", &cfg_json);
    }
    sync_config.set(cfg);
}

pub(super) fn build_sync_handlers(
    mut ctx: AppActionContext,
    on_refresh: EventHandler<()>,
) -> SyncAndDeviceHandlers {
    let on_export_to_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |(b_id, kind): (String, DeviceKind)| {
            let locale = *ctx.ui_locale.read();
            match ds.export_book_to_device(&b_id, kind, None) {
                Ok(_) => show_toast(
                    ctx.toast_msg,
                    translate_fmt(
                        locale,
                        "actions.device_exported",
                        &[("device", &format!("{kind:?}"))],
                    ),
                    true,
                ),
                Err(e) => {
                    tracing::error!("Device export error ({kind:?}): {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_sync_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |kind: DeviceKind| {
            let locale = *ctx.ui_locale.read();
            let _ = ds.sync_reading_progress(kind, None);
            let _ = ds.import_books_from_device(kind, None);
            on_refresh.call(());
            ctx.connected_devices.set(ds.detect_connected_devices());
            show_toast(
                ctx.toast_msg,
                translate_fmt(
                    locale,
                    "actions.device_synced",
                    &[("device", &format!("{kind:?}"))],
                ),
                true,
            );
        }
    });

    let on_sync_now = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let pr = ctx.book_service.param_repo().clone();
        move |()| {
            if *ctx.is_syncing.read() {
                return;
            }
            let locale = *ctx.ui_locale.read();
            ctx.is_syncing.set(true);
            ctx.sync_message
                .set(Some(translate(locale, "actions.sync_running").to_string()));
            let ss_cloned = ss.clone();
            let pr_cloned = pr.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let r = ss_cloned.sync_now();
                    let cfg = ss_cloned.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr_cloned.set_value("cloud_sync_config", &cfg_json);
                    }
                    (r, cfg)
                })
                .await;

                ctx.is_syncing.set(false);
                match res {
                    Ok((Ok(report), updated_cfg)) => {
                        ctx.sync_config.set(updated_cfg);
                        let msg = translate_fmt(
                            locale,
                            "actions.sync_ok",
                            &[
                                ("uploaded", &report.uploaded_books.to_string()),
                                ("downloaded", &report.downloaded_books.to_string()),
                                ("events", &report.pulled_events.to_string()),
                            ],
                        );
                        tracing::info!("{msg}");
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, true);
                        on_refresh.call(());
                    }
                    Ok((Err(e), _)) => {
                        let err_msg =
                            translate_fmt(locale, "actions.sync_err", &[("error", &e.to_string())]);
                        tracing::error!("{err_msg}");
                        ctx.sync_message.set(Some(err_msg.clone()));
                        show_toast(ctx.toast_msg, err_msg, false);
                    }
                    Err(e) => {
                        tracing::error!("Async sync task error: {e}");
                        let msg = translate(locale, "actions.sync_unexpected").to_string();
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, false);
                    }
                }
            });
        }
    });

    let on_pick_sync_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            if let Some(folder) = rfd::FileDialog::new()
                .set_title(translate(locale, "actions.sync_folder_title"))
                .pick_folder()
            {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        translate(locale, "actions.sync_folder_ok").to_string(),
                    ));
                }
            }
        }
    });

    let on_auto_detect_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            if let Some(folder) = FolderSyncBackend::auto_detect_google_drive() {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        translate(locale, "actions.sync_folder_detected").to_string(),
                    ));
                }
            } else {
                ctx.sync_message.set(Some(
                    translate(locale, "actions.sync_folder_not_found").to_string(),
                ));
            }
        }
    });

    let on_toggle_auto_download = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |auto_dl: bool| {
            ss.set_auto_download_books(auto_dl);
            persist_sync_config(&ss, &bs, ctx.sync_config);
        }
    });

    let on_toggle_sync_enabled = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |enabled: bool| {
            ss.set_enabled(enabled);
            persist_sync_config(&ss, &bs, ctx.sync_config);
        }
    });

    let on_set_sync_mode = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |mode: CloudSyncMode| {
            ss.set_sync_mode(mode);
            persist_sync_config(&ss, &bs, ctx.sync_config);
        }
    });

    let on_save_gdrive_creds = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |(client_id, client_secret, folder_name): (String, String, String)| {
            let locale = *ctx.ui_locale.read();
            let secret_opt = if client_secret.trim().is_empty() {
                None
            } else {
                Some(client_secret)
            };
            ss.set_google_drive_credentials(client_id, secret_opt, folder_name);
            persist_sync_config(&ss, &bs, ctx.sync_config);
            ctx.sync_message.set(Some(
                translate(locale, "actions.gdrive_creds_saved").to_string(),
            ));
        }
    });

    let on_connect_gdrive = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let pr = ctx.book_service.param_repo().clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            ctx.sync_message.set(Some(
                translate(locale, "settings.gdrive_connecting").to_string(),
            ));
            let ss_cloned = ss.clone();
            let pr_cloned = pr.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let r = ss_cloned.connect_google_drive();
                    let cfg = ss_cloned.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr_cloned.set_value("cloud_sync_config", &cfg_json);
                    }
                    (r, cfg)
                })
                .await;

                match res {
                    Ok((Ok(_email), updated_cfg)) => {
                        ctx.sync_config.set(updated_cfg);
                        let msg = translate(locale, "actions.gdrive_connected").to_string();
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, true);
                    }
                    Ok((Err(e), _)) => {
                        let msg = translate_fmt(
                            locale,
                            "actions.gdrive_connect_err",
                            &[("error", &e.to_string())],
                        );
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, false);
                    }
                    Err(e) => {
                        let msg = translate_fmt(
                            locale,
                            "actions.gdrive_connect_err",
                            &[("error", &e.to_string())],
                        );
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, false);
                    }
                }
            });
        }
    });

    let on_disconnect_gdrive = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            if ss.disconnect_google_drive().is_ok() {
                persist_sync_config(&ss, &bs, ctx.sync_config);
                let msg = translate(locale, "actions.gdrive_disconnected").to_string();
                ctx.sync_message.set(Some(msg.clone()));
                show_toast(ctx.toast_msg, msg, true);
            }
        }
    });

    let on_save_smtp = EventHandler::new({
        let pr = ctx.book_service.param_repo().clone();
        move |cfg: SmtpConfig| {
            ctx.smtp_config.set(cfg.clone());
            let secrets = SecretStore::default();
            if cfg.password.trim().is_empty() {
                let _ = secrets.delete_secret("smtp_password");
            } else {
                let _ = secrets.set_secret("smtp_password", &cfg.password);
            }
            let mut persisted_cfg = cfg;
            persisted_cfg.password.clear();
            if let Ok(cfg_json) = serde_json::to_string(&persisted_cfg) {
                let _ = pr.set_value("smtp_config", &cfg_json);
            }
        }
    });

    let on_test_smtp = EventHandler::new({
        let ms = ctx.mail_service.clone();
        move |cfg: SmtpConfig| {
            let locale = *ctx.ui_locale.read();
            ctx.is_testing_smtp.set(true);
            ctx.smtp_test_message.set(None);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.test_connection(&cfg).await;
                ctx.is_testing_smtp.set(false);
                match res {
                    Ok(()) => ctx.smtp_test_message.set(Some((
                        true,
                        translate(locale, "actions.smtp_ok").to_string(),
                    ))),
                    Err(e) => {
                        ctx.smtp_test_message.set(Some((
                            false,
                            translate_fmt(locale, "actions.smtp_err", &[("error", &e.to_string())]),
                        )));
                    }
                }
            });
        }
    });

    let on_send_to_kindle = EventHandler::new({
        let ms = ctx.mail_service.clone();
        move |b: Book| {
            let locale = *ctx.ui_locale.read();
            let cfg = ctx.smtp_config.read().clone();
            ctx.is_sending_kindle.set(true);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.send_to_kindle(&b, &cfg, None).await;
                ctx.is_sending_kindle.set(false);
                match res {
                    Ok(()) => show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "actions.kindle_sent", &[("title", &b.title)]),
                        true,
                    ),
                    Err(e) => {
                        tracing::error!("Kindle send error: {e}");
                        show_toast(
                            ctx.toast_msg,
                            translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                            false,
                        );
                    }
                }
            });
        }
    });

    SyncAndDeviceHandlers {
        on_export_to_device,
        on_sync_device,
        on_sync_now,
        on_pick_sync_folder,
        on_auto_detect_folder,
        on_toggle_auto_download,
        on_toggle_sync_enabled,
        on_set_sync_mode,
        on_save_gdrive_creds,
        on_connect_gdrive,
        on_disconnect_gdrive,
        on_save_smtp,
        on_test_smtp,
        on_send_to_kindle,
    }
}

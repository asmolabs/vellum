use dioxus::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use vellum_core::{
    translate, translate_fmt, ConnectedDevice, DeviceComparison, DeviceService, Locale,
};
use vellum_domain::DeviceKind;

use crate::components::{IconKind, LucideIcon};

#[component]
pub fn DeviceManagerModal(
    locale: Locale,
    device_service: Arc<DeviceService>,
    connected_devices: Signal<Vec<ConnectedDevice>>,
    on_close: EventHandler<()>,
    on_updated: EventHandler<()>,
) -> Element {
    let devs = connected_devices.read().clone();
    let initial_kind = devs
        .first()
        .map(|d| d.kind)
        .unwrap_or(DeviceKind::Generic);

    let mut selected_kind = use_signal(|| initial_kind);
    let mut custom_mount = use_signal(|| None::<PathBuf>);
    let mut comparison = use_signal(|| None::<DeviceComparison>);
    let mut is_loading = use_signal(|| false);
    let mut is_transferring = use_signal(|| false);
    let mut is_importing = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut success_msg = use_signal(|| None::<String>);
    let mut active_tab = use_signal(|| 0_usize); // 0: To Transfer, 1: On Device, 2: Only On Device
    let mut search_query = use_signal(String::new);
    let mut selected_transfer_ids = use_signal(HashSet::<String>::new);
    let mut selected_import_paths = use_signal(HashSet::<PathBuf>::new);

    let load_comparison = use_callback({
        let ds = device_service.clone();
        move |()| {
            let kind = *selected_kind.peek();
            let custom = custom_mount.peek().clone();
            is_loading.set(true);
            error_msg.set(None);
            let ds_task = ds.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    ds_task.compare_device(kind, custom.as_deref())
                })
                .await;
                is_loading.set(false);
                match res {
                    Ok(Ok(comp)) => {
                        let all_to_transfer: HashSet<String> = comp
                            .in_vellum_not_on_device
                            .iter()
                            .map(|b| b.id.clone())
                            .collect();
                        selected_transfer_ids.set(all_to_transfer);

                        let all_to_import: HashSet<PathBuf> = comp
                            .on_device_not_in_vellum
                            .iter()
                            .map(|d| d.relative_path.clone())
                            .collect();
                        selected_import_paths.set(all_to_import);

                        comparison.set(Some(comp));
                    }
                    Ok(Err(e)) => {
                        comparison.set(None);
                        error_msg.set(Some(e.to_string()));
                    }
                    Err(e) => {
                        comparison.set(None);
                        error_msg.set(Some(e.to_string()));
                    }
                }
            });
        }
    });

    // Chargement initial
    use_effect(move || {
        load_comparison.call(());
    });

    let do_transfer = use_callback({
        let ds = device_service.clone();
        move |()| {
            let kind = *selected_kind.peek();
            let custom = custom_mount.peek().clone();
            let ids: Vec<String> = selected_transfer_ids.peek().iter().cloned().collect();
            if ids.is_empty() {
                return;
            }
            is_transferring.set(true);
            error_msg.set(None);
            success_msg.set(None);
            let ds_task = ds.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    ds_task.export_books_batch(&ids, kind, custom.as_deref())
                })
                .await;
                is_transferring.set(false);
                match res {
                    Ok(Ok(count)) => {
                        success_msg.set(Some(translate_fmt(
                            locale,
                            "device_manager.transfer_success",
                            &[("count", &count.to_string())],
                        )));
                        load_comparison.call(());
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        error_msg.set(Some(e.to_string()));
                    }
                    Err(e) => {
                        error_msg.set(Some(e.to_string()));
                    }
                }
            });
        }
    });

    let do_import = use_callback({
        let ds = device_service.clone();
        move |()| {
            let kind = *selected_kind.peek();
            let custom = custom_mount.peek().clone();
            let paths: Vec<PathBuf> = selected_import_paths.peek().iter().cloned().collect();
            if paths.is_empty() {
                return;
            }
            is_importing.set(true);
            error_msg.set(None);
            success_msg.set(None);
            let ds_task = ds.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    ds_task.import_device_books_batch(&paths, kind, custom.as_deref())
                })
                .await;
                is_importing.set(false);
                match res {
                    Ok(Ok(imported)) => {
                        success_msg.set(Some(translate_fmt(
                            locale,
                            "device_manager.import_success",
                            &[("count", &imported.len().to_string())],
                        )));
                        load_comparison.call(());
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        error_msg.set(Some(e.to_string()));
                    }
                    Err(e) => {
                        error_msg.set(Some(e.to_string()));
                    }
                }
            });
        }
    });

    let cur_comp = comparison.read().clone();
    let q = search_query.read().trim().to_lowercase();

    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-card",
                style: "max-width: 980px; width: 94%; max-height: 88vh; display: flex; flex-direction: column; padding: 0; overflow: hidden; border-radius: 16px; background: var(--bg-surface); border: 1px solid var(--border-color); box-shadow: 0 20px 40px rgba(0,0,0,0.35);",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    style: "padding: 20px 24px; border-bottom: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center; background: var(--bg-surface-elevated, var(--bg-surface));",
                    div { style: "display: flex; align-items: center; gap: 12px;",
                        span {
                            style: "width: 40px; height: 40px; border-radius: 10px; background: var(--accent-glow); color: var(--accent-primary); display: inline-flex; align-items: center; justify-content: center;",
                            LucideIcon { kind: IconKind::BookOpen, size: 22 }
                        }
                        div {
                            h2 { style: "font-size: 17px; font-weight: 700; margin: 0 0 3px 0; color: var(--text-primary);",
                                {translate(locale, "device_manager.title")}
                            }
                            p { style: "font-size: 12px; margin: 0; color: var(--text-muted);",
                                {translate(locale, "device_manager.subtitle")}
                            }
                        }
                    }
                    button {
                        class: "btn-icon",
                        style: "width: 32px; height: 32px; border-radius: 8px; border: none; background: transparent; cursor: pointer; color: var(--text-muted); display: flex; align-items: center; justify-content: center;",
                        onclick: move |_| on_close.call(()),
                        LucideIcon { kind: IconKind::X, size: 18 }
                    }
                }

                // Device selector & Controls bar
                div {
                    style: "padding: 12px 24px; background: var(--bg-sidebar); border-bottom: 1px solid var(--border-color); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 13px; font-weight: 600; color: var(--text-secondary);",
                            {translate(locale, "device_manager.connected")}
                        }
                        select {
                            class: "input-field",
                            style: "padding: 5px 12px; font-size: 13px; font-weight: 500; border-radius: 8px; background: var(--bg-surface); color: var(--text-primary); border: 1px solid var(--border-color); cursor: pointer;",
                            value: selected_kind.read().as_str(),
                            onchange: move |evt: FormEvent| {
                                let v = evt.value();
                                let k = match v.as_str() {
                                    "kobo" => DeviceKind::Kobo,
                                    "kindle" => DeviceKind::Kindle,
                                    _ => DeviceKind::Generic,
                                };
                                selected_kind.set(k);
                                load_comparison.call(());
                            },
                            option { value: "generic", "📱 Xteink / Liseuse USB Générique / SD" }
                            option { value: "kobo", "📖 Kobo eReader (KEPUB)" }
                            option { value: "kindle", "📱 Amazon Kindle (AZW3/MOBI)" }
                        }

                        if let Some(ref mount) = *custom_mount.read() {
                            span {
                                style: "font-size: 12px; color: var(--accent-primary); background: var(--accent-glow); padding: 4px 10px; border-radius: 6px; font-family: monospace;",
                                "{mount.display()}"
                            }
                        } else if let Some(ref comp) = cur_comp {
                            span {
                                style: "font-size: 12px; color: var(--text-muted); background: var(--bg-surface); border: 1px solid var(--border-color); padding: 4px 10px; border-radius: 6px;",
                                "{comp.device.mount_point.display()}"
                            }
                        }
                    }

                    div { style: "display: flex; align-items: center; gap: 8px;",
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px; padding: 6px 12px; border-radius: 8px; display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| {
                                if let Some(folder) = crate::platform::pick_folder("Sélectionnez la liseuse ou le dossier monté") {
                                    custom_mount.set(Some(folder));
                                    load_comparison.call(());
                                }
                            },
                            LucideIcon { kind: IconKind::FolderPlus, size: 14 }
                            span { {translate(locale, "device_manager.select_custom_mount")} }
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px; padding: 6px 12px; border-radius: 8px; display: inline-flex; align-items: center; gap: 6px;",
                            disabled: *is_loading.read(),
                            onclick: move |_| load_comparison.call(()),
                            LucideIcon { kind: IconKind::RefreshCw, size: 14 }
                            span { {translate(locale, "device_manager.refresh")} }
                        }
                    }
                }

                // Alert messages
                if let Some(ref msg) = *success_msg.read() {
                    div {
                        style: "margin: 12px 24px 0; padding: 10px 14px; background: rgba(16, 185, 129, 0.12); border: 1px solid rgba(16, 185, 129, 0.3); border-radius: 8px; color: #10b981; font-size: 13px;",
                        "{msg}"
                    }
                }
                if let Some(ref msg) = *error_msg.read() {
                    div {
                        style: "margin: 12px 24px 0; padding: 10px 14px; background: rgba(239, 68, 68, 0.12); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 8px; color: #ef4444; font-size: 13px;",
                        "{msg}"
                    }
                }

                // Body content
                if *is_loading.read() {
                    div {
                        style: "padding: 60px; text-align: center; color: var(--text-muted); display: flex; flex-direction: column; align-items: center; gap: 12px;",
                        LucideIcon { kind: IconKind::RefreshCw, size: 28 }
                        span { style: "font-size: 14px;", {translate(locale, "device_manager.detecting")} }
                    }
                } else if cur_comp.is_none() {
                    div {
                        style: "padding: 60px 32px; text-align: center; color: var(--text-muted); display: flex; flex-direction: column; align-items: center; gap: 16px;",
                        LucideIcon { kind: IconKind::BookOpen, size: 40 }
                        h3 { style: "margin: 0; font-size: 16px; font-weight: 600; color: var(--text-primary);",
                            {translate(locale, "device_manager.no_device")}
                        }
                        p { style: "margin: 0; max-width: 500px; font-size: 13px; line-height: 1.5;",
                            {translate(locale, "device_manager.no_device_desc")}
                        }
                        button {
                            class: "btn btn-primary",
                            style: "margin-top: 8px; padding: 8px 18px; border-radius: 8px; font-size: 13px;",
                            onclick: move |_| {
                                if let Some(folder) = crate::platform::pick_folder("Sélectionnez la liseuse ou le dossier monté") {
                                    custom_mount.set(Some(folder));
                                    load_comparison.call(());
                                }
                            },
                            LucideIcon { kind: IconKind::FolderPlus, size: 16 }
                            span { style: "margin-left: 6px;", {translate(locale, "device_manager.select_custom_mount")} }
                        }
                    }
                } else if let Some(ref comp) = cur_comp {
                    // Tabs Header & Search
                    div {
                        style: "padding: 12px 24px; border-bottom: 1px solid var(--border-color); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;",
                        div { style: "display: flex; gap: 6px;",
                            button {
                                class: if *active_tab.read() == 0 { "btn btn-primary" } else { "btn btn-ghost" },
                                style: "font-size: 13px; padding: 6px 14px; border-radius: 8px;",
                                onclick: move |_| active_tab.set(0),
                                {translate_fmt(locale, "device_manager.tab_to_transfer", &[("count", &comp.in_vellum_not_on_device.len().to_string())])}
                            }
                            button {
                                class: if *active_tab.read() == 1 { "btn btn-primary" } else { "btn btn-ghost" },
                                style: "font-size: 13px; padding: 6px 14px; border-radius: 8px;",
                                onclick: move |_| active_tab.set(1),
                                {translate_fmt(locale, "device_manager.tab_on_device", &[("count", &comp.present_on_device.len().to_string())])}
                            }
                            button {
                                class: if *active_tab.read() == 2 { "btn btn-primary" } else { "btn btn-ghost" },
                                style: "font-size: 13px; padding: 6px 14px; border-radius: 8px;",
                                onclick: move |_| active_tab.set(2),
                                {translate_fmt(locale, "device_manager.tab_only_device", &[("count", &comp.on_device_not_in_vellum.len().to_string())])}
                            }
                        }

                        div { style: "position: relative; width: 260px;",
                            input {
                                r#type: "text",
                                class: "input-field",
                                style: "width: 100%; font-size: 12px; padding: 6px 12px; border-radius: 8px; border: 1px solid var(--border-color); background: var(--bg-surface); color: var(--text-primary);",
                                placeholder: translate(locale, "device_manager.search_placeholder"),
                                value: "{search_query.read()}",
                                oninput: move |evt: FormEvent| search_query.set(evt.value()),
                            }
                        }
                    }

                    // Tab 0: To Transfer
                    if *active_tab.read() == 0 {
                        {
                            let to_transfer: Vec<_> = comp.in_vellum_not_on_device
                                .iter()
                                .filter(|b| q.is_empty() || b.title.to_lowercase().contains(&q) || b.author.to_lowercase().contains(&q))
                                .cloned()
                                .collect();

                            let all_ids: HashSet<String> = to_transfer.iter().map(|b| b.id.clone()).collect();
                            let ids_for_select = all_ids.clone();

                            rsx! {
                                div { style: "display: flex; flex-direction: column; flex: 1; min-height: 0; overflow: hidden;",
                                    div {
                                        style: "padding: 10px 24px; background: var(--bg-surface-elevated, var(--bg-surface)); border-bottom: 1px solid var(--border-color); display: flex; align-items: center; justify-content: space-between;",
                                        div { style: "display: flex; align-items: center; gap: 10px;",
                                            button {
                                                class: "btn btn-ghost",
                                                style: "font-size: 12px; padding: 4px 8px;",
                                                onclick: move |_| selected_transfer_ids.set(ids_for_select.clone()),
                                                {translate(locale, "device_manager.select_all")}
                                            }
                                            span { style: "color: var(--border-color);", "|" }
                                            button {
                                                class: "btn btn-ghost",
                                                style: "font-size: 12px; padding: 4px 8px;",
                                                onclick: move |_| selected_transfer_ids.set(HashSet::new()),
                                                {translate(locale, "device_manager.deselect_all")}
                                            }
                                            span { style: "font-size: 12px; color: var(--text-muted); margin-left: 8px;",
                                                "({selected_transfer_ids.read().len()} sélectionnés)"
                                            }
                                        }

                                        button {
                                            class: "btn btn-primary",
                                            style: "font-size: 13px; padding: 6px 16px; border-radius: 8px; display: inline-flex; align-items: center; gap: 8px;",
                                            disabled: *is_transferring.read() || selected_transfer_ids.read().is_empty(),
                                            onclick: move |_| do_transfer.call(()),
                                            if *is_transferring.read() {
                                                LucideIcon { kind: IconKind::RefreshCw, size: 14 }
                                                span { {translate(locale, "device_manager.transferring")} }
                                            } else {
                                                LucideIcon { kind: IconKind::Upload, size: 14 }
                                                span { {translate_fmt(locale, "device_manager.transfer_btn", &[("count", &selected_transfer_ids.read().len().to_string())])} }
                                            }
                                        }
                                    }

                                    if to_transfer.is_empty() {
                                        div { style: "padding: 48px; text-align: center; color: var(--text-muted); font-size: 13px;",
                                            {translate(locale, "device_manager.empty_to_transfer")}
                                        }
                                    } else {
                                        div { style: "flex: 1; overflow-y: auto; max-height: 400px;",
                                            table { style: "width: 100%; border-collapse: collapse; font-size: 13px; text-align: left;",
                                                thead {
                                                    tr { style: "border-bottom: 1px solid var(--border-color); color: var(--text-muted); font-size: 12px;",
                                                        th { style: "width: 44px; padding: 10px 16px;", "" }
                                                        th { style: "padding: 10px 16px;", "Titre" }
                                                        th { style: "padding: 10px 16px;", "Auteur" }
                                                        th { style: "padding: 10px 16px; width: 120px;", "Format" }
                                                    }
                                                }
                                                tbody {
                                                    for b in to_transfer {
                                                        {
                                                            let b_id = b.id.clone();
                                                            let is_chk = selected_transfer_ids.read().contains(&b_id);
                                                            let fmt_str = b.formats.iter().map(|f| f.format.to_uppercase()).collect::<Vec<_>>().join(", ");
                                                            rsx! {
                                                                tr {
                                                                    key: "{b_id}",
                                                                    style: "border-bottom: 1px solid var(--border-color); cursor: pointer; transition: background 0.15s;",
                                                                    class: "book-row",
                                                                    onclick: {
                                                                        let id_toggle = b_id.clone();
                                                                        move |_| {
                                                                            let mut set = selected_transfer_ids.read().clone();
                                                                            if set.contains(&id_toggle) {
                                                                                set.remove(&id_toggle);
                                                                            } else {
                                                                                set.insert(id_toggle.clone());
                                                                            }
                                                                            selected_transfer_ids.set(set);
                                                                        }
                                                                    },
                                                                    td { style: "padding: 10px 16px; text-align: center;",
                                                                        input {
                                                                            r#type: "checkbox",
                                                                            checked: is_chk,
                                                                            onclick: move |e| e.stop_propagation(),
                                                                            onchange: {
                                                                                let id_c = b_id.clone();
                                                                                move |_| {
                                                                                    let mut set = selected_transfer_ids.read().clone();
                                                                                    if set.contains(&id_c) {
                                                                                        set.remove(&id_c);
                                                                                    } else {
                                                                                        set.insert(id_c.clone());
                                                                                    }
                                                                                    selected_transfer_ids.set(set);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    td { style: "padding: 10px 16px; font-weight: 500; color: var(--text-primary);",
                                                                        "{b.title}"
                                                                    }
                                                                    td { style: "padding: 10px 16px; color: var(--text-secondary);",
                                                                        "{b.author}"
                                                                    }
                                                                    td { style: "padding: 10px 16px; color: var(--text-muted); font-size: 12px;",
                                                                        "{fmt_str}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Tab 1: On Device
                    if *active_tab.read() == 1 {
                        {
                            let on_dev: Vec<_> = comp.present_on_device
                                .iter()
                                .filter(|d| q.is_empty() || d.title.to_lowercase().contains(&q) || d.author.to_lowercase().contains(&q))
                                .cloned()
                                .collect();

                            rsx! {
                                div { style: "flex: 1; min-height: 0; overflow-y: auto; max-height: 450px;",
                                    if on_dev.is_empty() {
                                        div { style: "padding: 48px; text-align: center; color: var(--text-muted); font-size: 13px;",
                                            {translate(locale, "device_manager.empty_on_device")}
                                        }
                                    } else {
                                        table { style: "width: 100%; border-collapse: collapse; font-size: 13px; text-align: left;",
                                            thead {
                                                tr { style: "border-bottom: 1px solid var(--border-color); color: var(--text-muted); font-size: 12px;",
                                                    th { style: "padding: 10px 20px;", "Titre" }
                                                    th { style: "padding: 10px 20px;", "Auteur" }
                                                    th { style: "padding: 10px 20px; width: 140px;", "Fichier" }
                                                    th { style: "padding: 10px 20px; width: 120px;", "Avancement" }
                                                }
                                            }
                                            tbody {
                                                for d in on_dev {
                                                    tr {
                                                        key: "{d.filename}",
                                                        style: "border-bottom: 1px solid var(--border-color);",
                                                        td { style: "padding: 10px 20px; font-weight: 500; color: var(--text-primary);",
                                                            "{d.title}"
                                                        }
                                                        td { style: "padding: 10px 20px; color: var(--text-secondary);",
                                                            "{d.author}"
                                                        }
                                                        td { style: "padding: 10px 20px; color: var(--text-muted); font-size: 11px; font-family: monospace;",
                                                            "{d.filename}"
                                                        }
                                                        td { style: "padding: 10px 20px; color: var(--accent-primary); font-size: 12px;",
                                                            if let Some(pct) = d.progress_percent {
                                                                "{pct * 100.0:.0}%"
                                                            } else {
                                                                "—"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Tab 2: Only on Device
                    if *active_tab.read() == 2 {
                        {
                            let only_dev: Vec<_> = comp.on_device_not_in_vellum
                                .iter()
                                .filter(|d| q.is_empty() || d.title.to_lowercase().contains(&q) || d.author.to_lowercase().contains(&q))
                                .cloned()
                                .collect();

                            let all_paths: HashSet<PathBuf> = only_dev.iter().map(|d| d.relative_path.clone()).collect();
                            let paths_for_select = all_paths.clone();

                            rsx! {
                                div { style: "display: flex; flex-direction: column; flex: 1; min-height: 0; overflow: hidden;",
                                    div {
                                        style: "padding: 10px 24px; background: var(--bg-surface-elevated, var(--bg-surface)); border-bottom: 1px solid var(--border-color); display: flex; align-items: center; justify-content: space-between;",
                                        div { style: "display: flex; align-items: center; gap: 10px;",
                                            button {
                                                class: "btn btn-ghost",
                                                style: "font-size: 12px; padding: 4px 8px;",
                                                onclick: move |_| selected_import_paths.set(paths_for_select.clone()),
                                                {translate(locale, "device_manager.select_all")}
                                            }
                                            span { style: "color: var(--border-color);", "|" }
                                            button {
                                                class: "btn btn-ghost",
                                                style: "font-size: 12px; padding: 4px 8px;",
                                                onclick: move |_| selected_import_paths.set(HashSet::new()),
                                                {translate(locale, "device_manager.deselect_all")}
                                            }
                                            span { style: "font-size: 12px; color: var(--text-muted); margin-left: 8px;",
                                                "({selected_import_paths.read().len()} sélectionnés)"
                                            }
                                        }

                                        button {
                                            class: "btn btn-secondary",
                                            style: "font-size: 13px; padding: 6px 16px; border-radius: 8px; display: inline-flex; align-items: center; gap: 8px;",
                                            disabled: *is_importing.read() || selected_import_paths.read().is_empty(),
                                            onclick: move |_| do_import.call(()),
                                            if *is_importing.read() {
                                                LucideIcon { kind: IconKind::RefreshCw, size: 14 }
                                                span { {translate(locale, "device_manager.importing")} }
                                            } else {
                                                LucideIcon { kind: IconKind::Download, size: 14 }
                                                span { {translate_fmt(locale, "device_manager.import_btn", &[("count", &selected_import_paths.read().len().to_string())])} }
                                            }
                                        }
                                    }

                                    if only_dev.is_empty() {
                                        div { style: "padding: 48px; text-align: center; color: var(--text-muted); font-size: 13px;",
                                            {translate(locale, "device_manager.empty_only_device")}
                                        }
                                    } else {
                                        div { style: "flex: 1; overflow-y: auto; max-height: 400px;",
                                            table { style: "width: 100%; border-collapse: collapse; font-size: 13px; text-align: left;",
                                                thead {
                                                    tr { style: "border-bottom: 1px solid var(--border-color); color: var(--text-muted); font-size: 12px;",
                                                        th { style: "width: 44px; padding: 10px 16px;", "" }
                                                        th { style: "padding: 10px 16px;", "Titre" }
                                                        th { style: "padding: 10px 16px;", "Auteur" }
                                                        th { style: "padding: 10px 16px; width: 140px;", "Fichier" }
                                                    }
                                                }
                                                tbody {
                                                    for d in only_dev {
                                                        {
                                                            let rel_p = d.relative_path.clone();
                                                            let is_chk = selected_import_paths.read().contains(&rel_p);
                                                            rsx! {
                                                                tr {
                                                                    key: "{d.filename}",
                                                                    style: "border-bottom: 1px solid var(--border-color); cursor: pointer;",
                                                                    class: "book-row",
                                                                    onclick: {
                                                                        let p_toggle = rel_p.clone();
                                                                        move |_| {
                                                                            let mut set = selected_import_paths.read().clone();
                                                                            if set.contains(&p_toggle) {
                                                                                set.remove(&p_toggle);
                                                                            } else {
                                                                                set.insert(p_toggle.clone());
                                                                            }
                                                                            selected_import_paths.set(set);
                                                                        }
                                                                    },
                                                                    td { style: "padding: 10px 16px; text-align: center;",
                                                                        input {
                                                                            r#type: "checkbox",
                                                                            checked: is_chk,
                                                                            onclick: move |e| e.stop_propagation(),
                                                                            onchange: {
                                                                                let p_c = rel_p.clone();
                                                                                move |_| {
                                                                                    let mut set = selected_import_paths.read().clone();
                                                                                    if set.contains(&p_c) {
                                                                                        set.remove(&p_c);
                                                                                    } else {
                                                                                        set.insert(p_c.clone());
                                                                                    }
                                                                                    selected_import_paths.set(set);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    td { style: "padding: 10px 16px; font-weight: 500; color: var(--text-primary);",
                                                                        "{d.title}"
                                                                    }
                                                                    td { style: "padding: 10px 16px; color: var(--text-secondary);",
                                                                        "{d.author}"
                                                                    }
                                                                    td { style: "padding: 10px 16px; color: var(--text-muted); font-size: 11px; font-family: monospace;",
                                                                        "{d.filename}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

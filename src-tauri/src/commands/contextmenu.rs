// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Native context menu implementation using Tauri v2's popup menu API.
// Replaces the show_context_menu stub with real native menus.

use serde::Deserialize;
use serde_json::Value;
use tauri::menu::*;
use tauri::Manager;

#[derive(Deserialize)]
struct ContextMenuItem {
    id: String,
    label: Option<String>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    submenu: Option<Vec<ContextMenuItem>>,
    checked: Option<bool>,
    visible: Option<bool>,
    enabled: Option<bool>,
}

#[tauri::command(rename_all = "camelCase")]
pub fn show_context_menu(window: tauri::Window, workspace_id: String, menu: Option<Value>) {
    let Some(menu_value) = menu else {
        tracing::debug!("show_context_menu: no menu provided for workspace={}", workspace_id);
        return;
    };

    let items: Vec<ContextMenuItem> = match serde_json::from_value(menu_value) {
        Ok(items) => items,
        Err(e) => {
            tracing::error!("show_context_menu: failed to deserialize menu: {}", e);
            return;
        }
    };

    let app = window.app_handle();
    let native_menu = match build_native_menu(app, &items) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("show_context_menu: failed to build menu: {}", e);
            return;
        }
    };

    if let Err(e) = native_menu.popup(window) {
        tracing::error!("show_context_menu: failed to show popup: {}", e);
    }
}

fn build_native_menu(
    app: &tauri::AppHandle,
    items: &[ContextMenuItem],
) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;

    for item in items {
        if item.visible == Some(false) {
            continue;
        }

        let item_type = item.item_type.as_deref().unwrap_or("normal");
        let label = item.label.as_deref().unwrap_or("");
        let enabled = item.enabled.unwrap_or(true);

        match item_type {
            "separator" => {
                menu.append(&PredefinedMenuItem::separator(app)?)?;
            }
            "submenu" => {
                let submenu = Submenu::with_id(app, &item.id, label, enabled)?;
                if let Some(children) = &item.submenu {
                    for child in children {
                        if child.visible == Some(false) {
                            continue;
                        }
                        append_item_to_submenu(app, &submenu, child)?;
                    }
                }
                menu.append(&submenu)?;
            }
            "checkbox" | "radio" => {
                let checked = item.checked.unwrap_or(false);
                let check_item =
                    CheckMenuItem::with_id(app, &item.id, label, enabled, checked, None::<&str>)?;
                menu.append(&check_item)?;
            }
            _ => {
                let menu_item =
                    MenuItem::with_id(app, &item.id, label, enabled, None::<&str>)?;
                menu.append(&menu_item)?;
            }
        }
    }

    Ok(menu)
}

fn append_item_to_submenu(
    app: &tauri::AppHandle,
    submenu: &Submenu<tauri::Wry>,
    item: &ContextMenuItem,
) -> tauri::Result<()> {
    let item_type = item.item_type.as_deref().unwrap_or("normal");
    let label = item.label.as_deref().unwrap_or("");
    let enabled = item.enabled.unwrap_or(true);

    match item_type {
        "separator" => {
            submenu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        "submenu" => {
            let child_submenu = Submenu::with_id(app, &item.id, label, enabled)?;
            if let Some(children) = &item.submenu {
                for child in children {
                    if child.visible == Some(false) {
                        continue;
                    }
                    append_item_to_submenu(app, &child_submenu, child)?;
                }
            }
            submenu.append(&child_submenu)?;
        }
        "checkbox" | "radio" => {
            let checked = item.checked.unwrap_or(false);
            let check_item =
                CheckMenuItem::with_id(app, &item.id, label, enabled, checked, None::<&str>)?;
            submenu.append(&check_item)?;
        }
        _ => {
            let menu_item =
                MenuItem::with_id(app, &item.id, label, enabled, None::<&str>)?;
            submenu.append(&menu_item)?;
        }
    }

    Ok(())
}

import reflex as rx
from ..state import AppState, PluginFieldModel

def edit_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("Éditer les métadonnées"),
            rx.dialog.description("Modifiez les informations du livre ci-dessous."),
            rx.vstack(
                rx.vstack(
                    rx.text("Titre", font_size="13px", font_weight="500"),
                    rx.input(
                        value=AppState.edit_title,
                        on_change=AppState.set_edit_title,
                        width="100%",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Auteur", font_size="13px", font_weight="500"),
                    rx.input(
                        value=AppState.edit_author,
                        on_change=AppState.set_edit_author,
                        width="100%",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Éditeur", font_size="13px", font_weight="500"),
                    rx.input(
                        value=AppState.edit_publisher,
                        on_change=AppState.set_edit_publisher,
                        width="100%",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Date de publication", font_size="13px", font_weight="500"),
                    rx.input(
                        value=AppState.edit_publish_date,
                        on_change=AppState.set_edit_publish_date,
                        width="100%",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Tags (séparés par des virgules)", font_size="13px", font_weight="500"),
                    rx.input(
                        value=AppState.edit_tags,
                        on_change=AppState.set_edit_tags,
                        width="100%",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Description", font_size="13px", font_weight="500"),
                    rx.text_area(
                        value=AppState.edit_description,
                        on_change=AppState.set_edit_description,
                        width="100%",
                        rows="4",
                    ),
                    width="100%",
                ),
                # Cover upload
                rx.vstack(
                    rx.text("Changer la couverture", font_size="13px", font_weight="500"),
                    rx.upload(
                        rx.text("📁 Glisser une image ou cliquer pour choisir"),
                        id="cover_upload",
                        accept={"image/*": [".jpg", ".jpeg", ".png", ".webp"]},
                        max_files=1,
                        on_drop=AppState.handle_cover_upload(rx.upload_files(upload_id="cover_upload")),
                        padding="3",
                        border="1px dashed gray",
                        border_radius="6px",
                        cursor="pointer",
                        width="100%",
                    ),
                    width="100%",
                ),
                spacing="3",
                width="100%",
                margin_y="4",
            ),
            rx.hstack(
                rx.button("Annuler", variant="soft", color_scheme="gray", on_click=AppState.close_edit_dialog, cursor="pointer"),
                rx.button("Enregistrer", on_click=AppState.save_metadata, color_scheme="purple", cursor="pointer"),
                justify="end",
                spacing="3",
                width="100%",
            ),
            max_width="500px",
        ),
        open=AppState.show_edit_dialog,
    )

def confirm_delete_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("Supprimer le livre"),
            rx.dialog.description("Êtes-vous sûr de vouloir supprimer ce livre définitivement ? Cette action est irréversible."),
            rx.hstack(
                rx.button("Annuler", variant="soft", color_scheme="gray", on_click=AppState.cancel_delete, cursor="pointer"),
                rx.button("Supprimer", color_scheme="red", on_click=AppState.confirm_delete_book, cursor="pointer"),
                justify="end",
                spacing="3",
                width="100%",
                margin_top="4",
            ),
            max_width="400px",
        ),
        open=AppState.show_confirm_delete,
    )

def render_plugin_field(plugin_id: rx.Var, field: PluginFieldModel) -> rx.Component:
    val = AppState.plugin_configs[plugin_id][field.key].to(str)

    return rx.vstack(
        rx.text(field.label, font_size="12px", font_weight="600"),
        rx.cond(
            field.type == "select",
            rx.select(
                field.options,
                value=val,
                on_change=lambda new_val: AppState.update_plugin_field(plugin_id, field.key, new_val),
                width="100%",
                size="1",
            ),
            rx.cond(
                field.type == "boolean",
                rx.hstack(
                    rx.switch(
                        checked=val == "true",
                        on_change=lambda checked: AppState.update_plugin_field(plugin_id, field.key, rx.cond(checked, "true", "false")),
                    ),
                    rx.text(rx.cond(val == "true", "Activé", "Désactivé"), font_size="12px"),
                    spacing="2",
                    align="center",
                ),
                rx.input(
                    value=val,
                    on_change=lambda new_val: AppState.update_plugin_field(plugin_id, field.key, new_val),
                    placeholder=field.default,
                    size="1",
                    width="100%",
                ),
            ),
        ),
        rx.cond(field.description != "", rx.text(field.description, font_size="10px", color="var(--text-muted)")),
        width="100%",
        spacing="1",
        margin_bottom="2",
    )

def settings_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("⚙️ Configuration & Paramètres"),
            rx.dialog.description("Gérez l'emplacement de votre bibliothèque, vos préférences et les configurations personnalisées de vos plugins."),
            rx.vstack(
                # Library Path Section
                rx.vstack(
                    rx.text("Emplacement de la bibliothèque", font_size="13px", font_weight="600"),
                    rx.box(
                        rx.text(
                            rx.cond(AppState.library_path != "", AppState.library_path, "Aucun dossier sélectionné"),
                            font_size="13px",
                            color="var(--text-secondary)",
                            style={"word-break": "break-all"},
                        ),
                        padding="3",
                        border_radius="8px",
                        border="1px solid var(--border-color)",
                        background="rgba(255, 255, 255, 0.03)",
                        width="100%",
                    ),
                    rx.button(
                        "📁 Changer le dossier de bibliothèque",
                        on_click=rx.call_script("window.pywebview ? window.pywebview.api.select_directory().then(path => path && reflex.AppState.set_library_path_action(path)) : prompt('Chemin du dossier:')"),
                        variant="outline",
                        width="100%",
                        cursor="pointer",
                    ),
                    width="100%",
                    spacing="2",
                ),
                # Theme Section
                rx.vstack(
                    rx.text("Apparence & Thème", font_size="13px", font_weight="600"),
                    rx.hstack(
                        rx.text("Thème actuel: ", font_size="13px"),
                        rx.text(rx.cond(AppState.theme == "light", "Mode Clair ☀️", "Mode Sombre 🌙"), font_size="13px", font_weight="600"),
                        rx.button(
                            rx.cond(AppState.theme == "light", "Passer en Sombre 🌙", "Passer en Clair ☀️"),
                            on_click=AppState.toggle_theme,
                            variant="soft",
                            size="1",
                            cursor="pointer",
                        ),
                        align="center",
                        justify="between",
                        width="100%",
                    ),
                    width="100%",
                    spacing="2",
                ),
                # Plugins Configuration Section (Saved in SQLite 'params' table as JSON)
                rx.vstack(
                    rx.text("🔌 Configuration des Plugins (Table SQLite 'params')", font_size="13px", font_weight="600"),
                    rx.foreach(
                        AppState.plugins,
                        lambda p: rx.box(
                            rx.vstack(
                                rx.hstack(
                                    rx.text("🔌", font_size="13px"),
                                    rx.text(p.name, font_size="13px", font_weight="700", color="#c084fc"),
                                    rx.badge(p.type, size="1", variant="soft"),
                                    align="center",
                                    spacing="2",
                                    width="100%",
                                ),
                                rx.text(p.description, font_size="11px", color="var(--text-muted)", margin_bottom="2"),
                                rx.foreach(
                                    p.config_schema,
                                    lambda f: render_plugin_field(p.id, f),
                                ),
                                rx.button(
                                    "💾 Enregistrer les paramètres du plugin",
                                    on_click=lambda: AppState.save_active_plugin_config(p.id),
                                    variant="soft",
                                    color_scheme="purple",
                                    size="1",
                                    width="100%",
                                    cursor="pointer",
                                ),
                                width="100%",
                                spacing="2",
                            ),
                            padding="3",
                            border_radius="8px",
                            border="1px solid var(--border-color)",
                            background="rgba(255, 255, 255, 0.02)",
                            margin_bottom="3",
                            width="100%",
                        ),
                    ),
                    rx.cond(
                        AppState.plugin_save_status != "",
                        rx.text(AppState.plugin_save_status, font_size="12px", font_weight="500", color="var(--success)"),
                    ),
                    width="100%",
                    spacing="2",
                ),
                spacing="4",
                width="100%",
                margin_y="4",
                max_height="60vh",
                overflow_y="auto",
            ),
            rx.hstack(
                rx.button("Fermer", on_click=AppState.close_settings_dialog, color_scheme="purple", cursor="pointer"),
                justify="end",
                width="100%",
            ),
            max_width="550px",
        ),
        open=AppState.show_settings_dialog,
    )

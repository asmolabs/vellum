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
                rx.hstack(
                    rx.vstack(
                        rx.text("Série / Saga", font_size="13px", font_weight="500"),
                        rx.input(
                            placeholder="ex: Dune",
                            value=AppState.edit_series,
                            on_change=AppState.set_edit_series,
                            width="100%",
                        ),
                        width="70%",
                    ),
                    rx.vstack(
                        rx.text("Tome n°", font_size="13px", font_weight="500"),
                        rx.input(
                            placeholder="1.0",
                            value=AppState.edit_series_index,
                            on_change=AppState.set_edit_series_index,
                            width="100%",
                        ),
                        width="30%",
                    ),
                    width="100%",
                    spacing="2",
                ),
                rx.hstack(
                    rx.vstack(
                        rx.text("ISBN", font_size="13px", font_weight="500"),
                        rx.input(
                            placeholder="978-...",
                            value=AppState.edit_isbn,
                            on_change=AppState.set_edit_isbn,
                            width="100%",
                        ),
                        width="60%",
                    ),
                    rx.vstack(
                        rx.text("Langue", font_size="13px", font_weight="500"),
                        rx.select(
                            ["fr", "en", "es", "de", "it"],
                            value=AppState.edit_language,
                            on_change=AppState.set_edit_language,
                            width="100%",
                        ),
                        width="40%",
                    ),
                    width="100%",
                    spacing="2",
                ),
                rx.vstack(
                    rx.text("Évaluation (Note)", font_size="13px", font_weight="500"),
                    rx.hstack(
                        rx.button("☆☆☆☆☆ (0)", size="1", variant=rx.cond(AppState.edit_rating == 0, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(0)),
                        rx.button("⭐ (1)", size="1", variant=rx.cond(AppState.edit_rating == 1, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(1)),
                        rx.button("⭐⭐ (2)", size="1", variant=rx.cond(AppState.edit_rating == 2, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(2)),
                        rx.button("⭐⭐⭐ (3)", size="1", variant=rx.cond(AppState.edit_rating == 3, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(3)),
                        rx.button("⭐⭐⭐⭐ (4)", size="1", variant=rx.cond(AppState.edit_rating == 4, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(4)),
                        rx.button("⭐⭐⭐⭐⭐ (5)", size="1", variant=rx.cond(AppState.edit_rating == 5, "solid", "soft"), on_click=lambda: AppState.set_edit_rating(5)),
                        spacing="1",
                        wrap="wrap",
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

def bulk_edit_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("📝 Édition par Lot (Bulk Edit)"),
            rx.dialog.description("Modifiez simultanément les métadonnées de tous les livres sélectionnés."),
            rx.vstack(
                rx.vstack(
                    rx.text("Nouvel Auteur (laisser vide pour ne pas modifier)", font_size="12px", font_weight="500"),
                    rx.input(
                        placeholder="ex: Frank Herbert",
                        value=AppState.bulk_author,
                        on_change=AppState.set_bulk_author,
                        width="100%",
                        size="1",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Nouvelle Série / Saga", font_size="12px", font_weight="500"),
                    rx.input(
                        placeholder="ex: Cycle de Dune",
                        value=AppState.bulk_series,
                        on_change=AppState.set_bulk_series,
                        width="100%",
                        size="1",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Attribuer des Tags (séparés par des virgules)", font_size="12px", font_weight="500"),
                    rx.input(
                        placeholder="ex: Science-Fiction, Favoris",
                        value=AppState.bulk_tags,
                        on_change=AppState.set_bulk_tags,
                        width="100%",
                        size="1",
                    ),
                    width="100%",
                ),
                rx.vstack(
                    rx.text("Nouvelle Évaluation (Note)", font_size="12px", font_weight="500"),
                    rx.hstack(
                        rx.button("Ne pas modifier", size="1", variant=rx.cond(AppState.bulk_rating == -1, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(-1)),
                        rx.button("⭐ (1)", size="1", variant=rx.cond(AppState.bulk_rating == 1, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(1)),
                        rx.button("⭐⭐ (2)", size="1", variant=rx.cond(AppState.bulk_rating == 2, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(2)),
                        rx.button("⭐⭐⭐ (3)", size="1", variant=rx.cond(AppState.bulk_rating == 3, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(3)),
                        rx.button("⭐⭐⭐⭐ (4)", size="1", variant=rx.cond(AppState.bulk_rating == 4, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(4)),
                        rx.button("⭐⭐⭐⭐⭐ (5)", size="1", variant=rx.cond(AppState.bulk_rating == 5, "solid", "soft"), on_click=lambda: AppState.set_bulk_rating(5)),
                        spacing="1",
                        wrap="wrap",
                    ),
                    width="100%",
                ),
                rx.cond(
                    AppState.bulk_status != "",
                    rx.text(AppState.bulk_status, font_size="12px", color="var(--danger)"),
                ),
                spacing="3",
                width="100%",
                margin_y="3",
            ),
            rx.hstack(
                rx.button("Annuler", variant="soft", color_scheme="gray", on_click=AppState.close_bulk_edit_dialog, cursor="pointer"),
                rx.button("Appliquer aux livres", on_click=AppState.save_bulk_edit, color_scheme="purple", cursor="pointer"),
                justify="end",
                spacing="3",
                width="100%",
            ),
            max_width="480px",
        ),
        open=AppState.show_bulk_edit_dialog,
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
                # OPDS Catalog Server Section
                rx.vstack(
                    rx.text("📡 Serveur Web OPDS 1.2 (Accès Distant & Mobile)", font_size="13px", font_weight="600"),
                    rx.box(
                        rx.hstack(
                            rx.text("URL du catalogue : ", font_size="12px"),
                            rx.text(AppState.opds_url, font_size="12px", color="#c084fc", font_weight="500", style={"word-break": "break-all"}),
                            spacing="1",
                        ),
                        rx.text(
                            "Entrez cette URL dans vos applications e-reader mobiles (KOReader, Moon+ Reader, KyBook, Stanza) pour télécharger vos livres à distance via le Wi-Fi.",
                            font_size="11px",
                            color="var(--text-muted)",
                            margin_top="4px",
                        ),
                        padding="3",
                        border_radius="8px",
                        border="1px solid var(--border-color)",
                        background="rgba(255, 255, 255, 0.03)",
                        width="100%",
                    ),
                    width="100%",
                    spacing="2",
                ),

                # SMTP Kindle Settings Section
                rx.vstack(
                    rx.text("📧 Configuration SMTP (Send-to-Kindle)", font_size="13px", font_weight="600"),
                    rx.hstack(
                        rx.input(
                            placeholder="Serveur SMTP (ex: smtp.gmail.com)",
                            value=AppState.smtp_host,
                            on_change=AppState.set_smtp_host,
                            size="1",
                            width="70%",
                        ),
                        rx.input(
                            placeholder="Port (587)",
                            value=AppState.smtp_port.to(str),
                            on_change=AppState.set_smtp_port,
                            size="1",
                            width="30%",
                        ),
                        width="100%",
                        spacing="2",
                    ),
                    rx.hstack(
                        rx.input(
                            placeholder="Utilisateur SMTP / Email",
                            value=AppState.smtp_user,
                            on_change=AppState.set_smtp_user,
                            size="1",
                            width="50%",
                        ),
                        rx.input(
                            placeholder="Mot de passe SMTP / App",
                            type="password",
                            value=AppState.smtp_pass,
                            on_change=AppState.set_smtp_pass,
                            size="1",
                            width="50%",
                        ),
                        width="100%",
                        spacing="2",
                    ),
                    width="100%",
                    spacing="2",
                ),
                # Plugins Configuration Section (Saved in SQLite 'params' table as JSON)
                rx.vstack(
                    rx.text("🧩 Plugins & Extensions par Catégories (Table SQLite 'params')", font_size="13px", font_weight="600"),
                    rx.foreach(
                        AppState.plugins,
                        lambda p: rx.box(
                            rx.vstack(
                                rx.hstack(
                                    rx.text("🔌", font_size="13px"),
                                    rx.text(p.name, font_size="13px", font_weight="700", color="#c084fc"),
                                    rx.badge(
                                        p.category.upper(), 
                                        size="1", 
                                        variant="soft",
                                        color_scheme=rx.cond(
                                            p.category == "import", "green",
                                            rx.cond(
                                                p.category == "metadata", "blue",
                                                rx.cond(
                                                    p.category == "liseuse", "amber",
                                                    rx.cond(
                                                        p.category == "export", "cyan",
                                                        "purple"
                                                    )
                                                )
                                            )
                                        )
                                    ),
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
                                    "💾 Enregistrer la configuration du plugin",
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

def stats_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("📊 Statistiques Avancées de la Bibliothèque"),
            rx.dialog.description("Aperçu de la taille, des lectures et de la répartition de vos ouvrages."),
            rx.vstack(
                rx.hstack(
                    rx.box(
                        rx.text("📚 Total Livres", font_size="11px", color="var(--text-muted)"),
                        rx.heading(AppState.analytics_data.total_books.to(str), size="4", color="#c084fc"),
                        padding="3", border_radius="8px", border="1px solid var(--border-color)", flex="1",
                    ),
                    rx.box(
                        rx.text("💾 Taille Totale", font_size="11px", color="var(--text-muted)"),
                        rx.hstack(
                            rx.heading(AppState.analytics_data.total_size_mb.to(str), size="4", color="#38bdf8"),
                            rx.text(" MB", font_size="14px", font_weight="600"),
                            align="baseline",
                        ),
                        padding="3", border_radius="8px", border="1px solid var(--border-color)", flex="1",
                    ),
                    rx.box(
                        rx.text("✅ Livres Lus", font_size="11px", color="var(--text-muted)"),
                        rx.heading(AppState.analytics_data.read_count.to(str), size="4", color="#4ade80"),
                        padding="3", border_radius="8px", border="1px solid var(--border-color)", flex="1",
                    ),
                    width="100%",
                    spacing="3",
                ),
                rx.vstack(
                    rx.text("Auteurs les plus représentés", font_size="13px", font_weight="600"),
                    rx.foreach(
                        AppState.analytics_data.top_authors,
                        lambda item: rx.hstack(
                            rx.text(item.author, font_size="12px", flex="1"),
                            rx.badge(item.count.to(str), " livre(s)", size="1", variant="soft"),
                            width="100%",
                        ),
                    ),
                    width="100%",
                    spacing="2",
                    margin_top="3",
                ),
                spacing="4",
                width="100%",
                margin_y="4",
            ),
            rx.hstack(
                rx.button("Fermer", on_click=AppState.close_stats_dialog, color_scheme="purple", cursor="pointer"),
                justify="end",
                width="100%",
            ),
            max_width="500px",
        ),
        open=AppState.show_stats_dialog,
    )

def duplicates_dialog() -> rx.Component:
    return rx.dialog.root(
        rx.dialog.content(
            rx.dialog.title("🔍 Détecteur de Doublons"),
            rx.dialog.description("Analyse des doublons détectés dans votre collection."),
            rx.vstack(
                rx.cond(
                    AppState.duplicates_data.length() == 0,
                    rx.text("✅ Aucun doublon détecté dans votre bibliothèque !", font_size="13px", color="var(--success)"),
                    rx.foreach(
                        AppState.duplicates_data,
                        lambda item: rx.box(
                            rx.hstack(
                                rx.text("⚠️ Doublon trouvé : ", font_weight="600", font_size="13px"),
                                rx.text(item.duplicate.title, font_weight="600", font_size="13px"),
                                spacing="1",
                            ),
                            rx.hstack(
                                rx.text("Auteur: ", font_size="12px", color="var(--text-muted)"),
                                rx.text(item.duplicate.author, font_size="12px", color="var(--text-muted)"),
                                spacing="1",
                            ),
                            padding="3",
                            border_radius="6px",
                            border="1px solid var(--border-color)",
                            width="100%",
                        ),
                    ),
                ),
                spacing="3",
                width="100%",
                margin_y="4",
            ),
            rx.hstack(
                rx.button("Fermer", on_click=AppState.close_duplicates_dialog, color_scheme="purple", cursor="pointer"),
                justify="end",
                width="100%",
            ),
            max_width="480px",
        ),
        open=AppState.show_duplicates_dialog,
    )

//! Icônes SVG vectorielles monochromes style Lucide (inspirées de BookOrbit).

use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IconKind {
    Home,
    LayoutDashboard,
    Library,
    BookOpen,
    CheckCircle,
    Bookmark,
    Layers,
    Users,
    History,
    Tag,
    BarChart,
    PanelLeft,
    Search,
    Settings,
    Plus,
    Cloud,
    Wrench,
    ChevronDown,
    ChevronRight,
    Grid,
    List,
    Columns,
    Headphones,
    Image,
}

#[component]
pub fn LucideIcon(kind: IconKind, #[props(default = 16)] size: u32) -> Element {
    let s = size.to_string();
    rsx! {
        svg {
            width: "{s}",
            height: "{s}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            style: "flex-shrink: 0; display: inline-block; vertical-align: middle;",
            match kind {
                IconKind::Home => rsx! {
                    path { d: "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8" }
                    path { d: "M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
                },
                IconKind::LayoutDashboard => rsx! {
                    rect { width: "7", height: "9", x: "3", y: "3", rx: "1" }
                    rect { width: "7", height: "5", x: "14", y: "3", rx: "1" }
                    rect { width: "7", height: "9", x: "14", y: "12", rx: "1" }
                    rect { width: "7", height: "5", x: "3", y: "16", rx: "1" }
                },
                IconKind::Users => rsx! {
                    path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
                    circle { cx: "9", cy: "7", r: "4" }
                    path { d: "M22 21v-2a4 4 0 0 0-3-3.87" }
                    path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
                },
                IconKind::History => rsx! {
                    path { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" }
                    path { d: "M3 3v5h5" }
                    path { d: "M12 7v5l4 2" }
                },
                IconKind::Library => rsx! {
                    path { d: "m16 6 4 14" }
                    path { d: "M12 6v14" }
                    path { d: "M8 8v12" }
                    path { d: "M4 4v16" }
                },
                IconKind::BookOpen => rsx! {
                    path { d: "M12 7v14" }
                    path { d: "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z" }
                },
                IconKind::CheckCircle => rsx! {
                    circle { cx: "12", cy: "12", r: "10" }
                    path { d: "m9 12 2 2 4-4" }
                },
                IconKind::Bookmark => rsx! {
                    path { d: "m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z" }
                },
                IconKind::Layers => rsx! {
                    path { d: "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z" }
                    path { d: "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12" }
                    path { d: "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17" }
                },
                IconKind::Tag => rsx! {
                    path { d: "M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z" }
                    circle { cx: "7.5", cy: "7.5", r: ".5", fill: "currentColor" }
                },
                IconKind::BarChart => rsx! {
                    path { d: "M3 3v16a2 2 0 0 0 2 2h16" }
                    path { d: "M18 17V9" }
                    path { d: "M13 17V5" }
                    path { d: "M8 17v-3" }
                },
                IconKind::PanelLeft => rsx! {
                    rect { width: "18", height: "18", x: "3", y: "3", rx: "2" }
                    path { d: "M9 3v18" }
                },
                IconKind::Search => rsx! {
                    circle { cx: "11", cy: "11", r: "8" }
                    path { d: "m21 21-4.3-4.3" }
                },
                IconKind::Settings => rsx! {
                    path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
                    circle { cx: "12", cy: "12", r: "3" }
                },
                IconKind::Plus => rsx! {
                    path { d: "M5 12h14" }
                    path { d: "M12 5v14" }
                },
                IconKind::Cloud => rsx! {
                    path { d: "M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" }
                },
                IconKind::Wrench => rsx! {
                    path { d: "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" }
                },
                IconKind::ChevronDown => rsx! {
                    path { d: "m6 9 6 6 6-6" }
                },
                IconKind::ChevronRight => rsx! {
                    path { d: "m9 18 6-6-6-6" }
                },
                IconKind::Grid => rsx! {
                    rect { width: "7", height: "7", x: "3", y: "3", rx: "1" }
                    rect { width: "7", height: "7", x: "14", y: "3", rx: "1" }
                    rect { width: "7", height: "7", x: "14", y: "14", rx: "1" }
                    rect { width: "7", height: "7", x: "3", y: "14", rx: "1" }
                },
                IconKind::List => rsx! {
                    path { d: "M8 6h13" }
                    path { d: "M8 12h13" }
                    path { d: "M8 18h13" }
                    path { d: "M3 6h.01" }
                    path { d: "M3 12h.01" }
                    path { d: "M3 18h.01" }
                },
                IconKind::Columns => rsx! {
                    rect { width: "18", height: "18", x: "3", y: "3", rx: "2" }
                    path { d: "M9 3v18" }
                    path { d: "M15 3v18" }
                },
                IconKind::Headphones => rsx! {
                    path { d: "M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3" }
                },
                IconKind::Image => rsx! {
                    rect { width: "18", height: "18", x: "3", y: "3", rx: "2", ry: "2" }
                    circle { cx: "9", cy: "9", r: "2" }
                    path { d: "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" }
                },
            }
        }
    }
}

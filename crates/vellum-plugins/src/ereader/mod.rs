pub mod clippings;
pub mod generic;
pub mod kepubify;
pub mod kindle;
pub mod kobo;

pub use clippings::parse_kindle_clippings;
pub use generic::GenericSyncPlugin;
pub use kepubify::convert_epub_to_kepub;
pub use kindle::KindleSyncPlugin;
pub use kobo::KoboSyncPlugin;

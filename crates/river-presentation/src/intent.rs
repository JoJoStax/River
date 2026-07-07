use river_core::{MediaCategory, MediaItem, PluginId};

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    SelectCategory(MediaCategory),
    LoadCatalogs(MediaCategory),
    Search { query: String },
    GetDetails { plugin_id: PluginId, item_id: String },
    ReadChapter { plugin_id: PluginId, chapter_id: String },
    AddToLibrary(MediaItem),
    RemoveFromLibrary { plugin_id: PluginId, item_id: String },
    LoadLibrary(Option<MediaCategory>),
    ListPlugins,
    TogglePlugin { plugin_id: PluginId, enable: bool },
}

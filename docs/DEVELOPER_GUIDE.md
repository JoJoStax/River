# River Developer Guide

Welcome to the **River Developer Guide**! River is an ultra plugin-focused media aggregator written purely in Rust. Its mission is to replace specialized apps (Stremio for video, Spotify/Apple Music for music, Tachiyomi/Mihon for manga, and podcast apps) with a single, highly customizable, modular hub.

This manual is designed to teach you how River works under the hood and how you can program alternative plugins, swap out infrastructure adapters, and build custom user interfaces for Linux and Android—all using **100% Pure Rust** without any web elements or HTTP API servers.

---

## 1. Architectural Philosophy: Why Hexagonal & MVI?

Traditional MVC (Model-View-Controller) architectures tend to break down or become bloated when building apps across disparate platforms (Linux desktop, Windows, Android, Android TV). To avoid this, River uses two cutting-edge architectural patterns:

### Hexagonal Architecture (Ports & Adapters)
We strictly separate the codebase into concentric layers:
1. **Domain Core (`river-core`)**: Contains pure data structures (`MediaItem`, `Catalog`, `MediaStream`, `MangaChapter`, `MangaPage`), value objects (`MediaCategory`, `PluginId`), and error types. It has **zero dependencies** on databases, network libraries, or UI frameworks.
2. **Ports (`river-ports`)**: Defines pure Rust trait boundaries:
   - **Inbound Ports (Use Cases / What UI calls)**: `CatalogPort`, `LibraryPort`, `PluginPort`.
   - **Outbound Ports (Spi / What adapters implement)**: `PluginProvider`, `StorageRepository`, `NetworkClient`.
3. **Services (`river-services`)**: Implements the Inbound Ports by orchestrating Outbound Ports (e.g., querying multiple plugins in parallel, deduplicating search results, tracking watch progress).
4. **Adapters (`river-adapters/*`)**: Interchangeable infrastructure modules where network libraries (`reqwest`) and databases (`rusqlite`) live.

### MVI (Model-View-Intent) State Engine
Instead of brittle UI controllers, our presentation layer (`river-presentation`) uses a unidirectional data flow:
- **Intent**: An immutable action representing user intent (e.g., `Intent::Search("anime")`, `Intent::SelectCategory(MediaCategory::Manga)`).
- **Store (`AppStore`)**: Receives an Intent, executes business logic asynchronously via Inbound Ports, and updates an immutable state snapshot.
- **State (`AppState`)**: A clean snapshot of what the UI should render (`CatalogState`, `SearchState`, `DetailsState`, `ReaderState`, `LibraryState`). Any UI simply reads this state and draws!

---

## 2. Workspace Crate Breakdown

```
river/
├── crates/
│   ├── river-core/               # Layer 1: Domain models & error definitions
│   ├── river-ports/              # Layer 2: Inbound & Outbound trait boundaries
│   ├── river-services/           # Layer 3: Catalog, Library, and Plugin use cases
│   ├── river-adapters/           # Layer 4: Infrastructure & External integrations
│   │   ├── storage-sqlite/       #   - Bundled SQLite library & progress storage
│   │   ├── network-reqwest/      #   - Async HTTP network client
│   │   ├── plugin-stremio/       #   - Video adapter (Stremio Cinemeta & public movies)
│   │   ├── plugin-mangadex/      #   - Manga adapter (MangaDex REST API & comics)
│   │   ├── plugin-jamendo/       #   - Music adapter (Creative Commons audio streams)
│   │   └── plugin-rss/           #   - Podcast adapter (RSS podcast directories)
│   ├── river-presentation/       # Layer 5: MVI State Engine (Intent, State, AppStore)
│   ├── river-engine/             # Layer 6: Dependency injection & composition root
│   ├── river-cli/                # Layer 7: Interactive terminal verification runner
│   └── river-app/                # Layer 8: Pure Rust Linux & Android GUI (eframe/egui)
```

---

## 3. Tutorial: How to Write a Custom Plugin

Writing a plugin in River is as simple as creating a Rust struct and implementing the `PluginProvider` trait from `river-ports`.

### Step 1: Define your Plugin Struct
Create a new crate or file (e.g., `my_custom_plugin.rs`):

```rust
use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, Result, Subtitle,
};
use river_ports::{NetworkClient, PluginProvider};
use std::sync::Arc;

pub struct MyCustomVideoPlugin {
    client: Arc<dyn NetworkClient>,
}

impl MyCustomVideoPlugin {
    pub fn new(client: Arc<dyn NetworkClient>) -> Self {
        Self { client }
    }
}
```

### Step 2: Implement `PluginProvider`
Implement the required methods. If your plugin only supports Video, you can return empty vectors for manga chapters and pages!

```rust
#[async_trait]
impl PluginProvider for MyCustomVideoPlugin {
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            id: PluginId("my-custom-video".to_string()),
            name: "My Custom Video Scraper".to_string(),
            version: "1.0.0".to_string(),
            author: "Your Name".to_string(),
            description: "Scrapes custom video feeds and APIs.".to_string(),
            supported_categories: vec![MediaCategory::Video],
            icon_url: None,
        }
    }

    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>> {
        if req.category != MediaCategory::Video {
            return Ok(vec![]);
        }

        // Use self.client to grab external JSON APIs!
        // let json: serde_json::Value = self.client.get_json("https://api.example.com/movies").await?;

        let item = MediaItem {
            id: "movie-101".to_string(),
            plugin_id: self.metadata().id,
            category: MediaCategory::Video,
            title: "Open Source Movie".to_string(),
            description: Some("A fantastic open-source film.".to_string()),
            poster_url: Some("https://example.com/poster.jpg".to_string()),
            backdrop_url: None,
            year: Some(2026),
            genres: vec!["Sci-Fi".to_string()],
            author_or_creator: Some("Open Cinema".to_string()),
            rating: Some(9.5),
        };

        Ok(vec![Catalog {
            id: "my-catalog".to_string(),
            name: "Featured Movies".to_string(),
            category: MediaCategory::Video,
            items: vec![item],
        }])
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        // Implement search logic here
        Ok(vec![])
    }

    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem> {
        // Fetch full metadata for item_id
        Err(river_core::RiverError::NotFound(item_id.to_string()))
    }

    async fn get_streams(&self, item_id: &str) -> Result<Vec<MediaStream>> {
        Ok(vec![MediaStream {
            id: "stream-1".to_string(),
            title: "1080p Direct Stream".to_string(),
            url: "https://example.com/video.mp4".to_string(),
            quality: Some("1080p".to_string()),
            is_direct: true,
            is_hls_or_dash: false,
            is_magnet_or_torrent: false,
            headers: None,
        }])
    }

    async fn get_manga_chapters(&self, _manga_id: &str) -> Result<Vec<MangaChapter>> { Ok(vec![]) }
    async fn get_manga_pages(&self, _chapter_id: &str) -> Result<Vec<MangaPage>> { Ok(vec![]) }
    async fn get_subtitles(&self, _item_id: &str) -> Result<Vec<Subtitle>> { Ok(vec![]) }
}
```

### Step 3: Register your Plugin
In `river-engine/src/lib.rs` (or in your app initialization), register your plugin:

```rust
let my_plugin = Arc::new(MyCustomVideoPlugin::new(network_client.clone()));
plugin_service.register_provider(my_plugin).await;
```
That's it! Your plugin will now be automatically queried in parallel during catalog aggregation and search across Linux, Android, and CLI!

---

## 4. How to Swap Out Infrastructure Adapters

Because River uses Hexagonal Architecture, you can replace our SQLite storage or Reqwest network client without changing a single line of domain or service code!

### Swapping Storage (e.g., to pure Rust `redb` or PostgreSQL)
1. Create a struct implementing `river_ports::StorageRepository`.
2. Implement methods: `save_library_item()`, `get_library_items()`, `save_watch_progress()`, etc.
3. Pass your custom storage instance into `PluginService` and `LibraryService` inside `RiverEngine`!

---

## 5. Using the MVI State Engine in Custom Frontends

When building a custom UI or scripting tool, you interact exclusively with `AppStore` from `river-presentation`:

```rust
// 1. Initialize engine and get store
let engine = RiverEngine::new_with_db_path("my_library.db").await?;
let store = &engine.store;

// 2. Dispatch an Intent (asynchronously triggers background use cases)
store.dispatch(Intent::LoadCatalogs(MediaCategory::Video)).await;

// 3. Read immutable State snapshot
let state = store.get_state().await;
match state.catalog_state {
    CatalogState::Loaded(catalogs) => {
        for cat in catalogs {
            println!("Catalog: {}", cat.name);
        }
    }
    CatalogState::Loading => println!("Loading..."),
    CatalogState::Error(e) => println!("Error: {}", e),
    _ => {}
}
```

---

## 6. Building for Linux Desktop & Android

River's GUI (`river-app`) is built using **`eframe` (egui)**—an immediate-mode GUI framework written in 100% pure Rust with zero webviews or HTTP servers.

### Running on Linux Desktop
To launch the native window on Linux X11/Wayland:
```bash
cargo run -p river-app
```

### Compiling for Android
Because `eframe` renders natively via OpenGL/Vulkan/WGPU without a webview, it compiles directly to Android NDK APKs!

1. Install Android NDK and target architectures:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   cargo install cargo-apk
   ```
2. In `crates/river-app/src/android.rs`, we expose the standard Android entrypoint:
   ```rust
   #[no_mangle]
   pub extern "C" fn android_main(app: winit::platform::android::activity::AndroidApp) {
       // Initializes eframe native Android rendering!
   }
   ```
3. Build and install the APK to your connected Android phone or emulator:
   ```bash
   cargo apk run -p river-app
   ```
This will build a native APK, install it on your Android device, and launch River immediately!

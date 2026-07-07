use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use river_core::{MediaCategory, PluginId};
use river_engine::RiverEngine;
use river_presentation::{
    CatalogState, DetailsState, Intent, LibraryState, ReaderState, SearchState,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "river-cli",
    about = "River: Ultra Plugin-Focused Media Aggregator (Pure Rust Backend)",
    version
)]
struct Cli {
    #[arg(short, long, default_value = "river_library.db")]
    db_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all registered media plugins and their capabilities
    Plugins,

    /// Fetch and display aggregated catalogs for a media category (video, music, manga, podcast)
    Catalog {
        /// Media category: video, music, manga, or podcast
        category: String,
    },

    /// Search across all enabled plugins simultaneously
    Search {
        /// Search query string
        query: String,
    },

    /// Get detailed metadata, playable streams, and manga chapters for a specific item
    Details {
        /// Plugin ID (e.g., stremio-cinemeta, mangadex, jamendo-music, rss-podcasts)
        plugin_id: String,
        /// Item ID
        item_id: String,
    },

    /// Read manga pages for a specific chapter
    Read {
        /// Plugin ID
        plugin_id: String,
        /// Chapter ID
        chapter_id: String,
    },

    /// View items stored in the local SQLite library
    Library {
        /// Optional category filter
        category: Option<String>,
    },

    /// Add an item from a catalog to the local SQLite library
    LibraryAdd {
        /// Plugin ID
        plugin_id: String,
        /// Item ID to add
        item_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .init();

    let cli = Cli::parse();
    let engine = RiverEngine::new_with_db_path(&cli.db_path)
        .await
        .context("Failed to initialize RiverEngine")?;

    match cli.command {
        Commands::Plugins => {
            engine.store.dispatch(Intent::ListPlugins).await;
            let state = engine.store.get_state().await;
            println!("\n=== REGISTERED RIVER PLUGINS ===");
            for p in &state.plugins {
                println!(
                    "[{}] {} (v{}) - by {}",
                    p.id, p.name, p.version, p.author
                );
                println!("    Categories: {:?}", p.supported_categories);
                println!("    Description: {}\n", p.description);
            }
        }
        Commands::Catalog { category } => {
            let cat = MediaCategory::from_str(&category)
                .ok_or_else(|| anyhow::anyhow!("Invalid category: {}. Valid categories are: video, music, manga, podcast", category))?;
            
            println!("\n=== FETCHING CATALOG FOR: {} ===", cat.to_string().to_uppercase());
            engine.store.dispatch(Intent::LoadCatalogs(cat)).await;
            let state = engine.store.get_state().await;

            match state.catalog_state {
                CatalogState::Loaded(catalogs) => {
                    for cat in catalogs {
                        println!("\n--- Catalog: {} [{}] ---", cat.name, cat.id);
                        for item in cat.items {
                            println!(
                                "  * [{}] {} (by {:?}) - Plugin: {}",
                                item.id,
                                item.title,
                                item.author_or_creator.as_deref().unwrap_or("Unknown"),
                                item.plugin_id
                            );
                            if let Some(desc) = item.description {
                                let short_desc: String = desc.chars().take(80).collect();
                                println!("      Description: {}...", short_desc);
                            }
                        }
                    }
                }
                CatalogState::Error(e) => println!("Error loading catalog: {}", e),
                _ => println!("Catalog state: {:?}", state.catalog_state),
            }
        }
        Commands::Search { query } => {
            println!("\n=== SEARCHING ACROSS ALL PLUGINS FOR: \"{}\" ===", query);
            engine.store.dispatch(Intent::Search { query: query.clone() }).await;
            let state = engine.store.get_state().await;

            match state.search_state {
                SearchState::Results(items) => {
                    if items.is_empty() {
                        println!("No items found matching \"{}\".", query);
                    } else {
                        for item in items {
                            println!(
                                "  * [{}] {} ({:?}) - Plugin: {} | Category: {}",
                                item.id,
                                item.title,
                                item.year.unwrap_or(0),
                                item.plugin_id,
                                item.category
                            );
                        }
                    }
                }
                SearchState::Error(e) => println!("Error searching: {}", e),
                _ => println!("Search state: {:?}", state.search_state),
            }
        }
        Commands::Details { plugin_id, item_id } => {
            println!("\n=== ITEM DETAILS: {} (Plugin: {}) ===", item_id, plugin_id);
            engine
                .store
                .dispatch(Intent::GetDetails {
                    plugin_id: PluginId(plugin_id.clone()),
                    item_id: item_id.clone(),
                })
                .await;
            let state = engine.store.get_state().await;

            match state.details_state {
                DetailsState::Loaded { item, streams, chapters } => {
                    println!("Title: {}", item.title);
                    println!("Category: {}", item.category);
                    println!("Genres: {:?}", item.genres);
                    if let Some(desc) = item.description {
                        println!("Description: {}", desc);
                    }
                    if !streams.is_empty() {
                        println!("\nPlayable Streams:");
                        for s in streams {
                            println!("  -> [{}] {} (Quality: {:?}) - URL: {}", s.id, s.title, s.quality, s.url);
                        }
                    }
                    if !chapters.is_empty() {
                        println!("\nManga Chapters (Total: {}):", chapters.len());
                        for c in chapters.iter().take(10) {
                            println!("  -> [{}] Ch. {} - {}", c.id, c.chapter_number, c.title.as_deref().unwrap_or(""));
                        }
                        if chapters.len() > 10 {
                            println!("  ... and {} more chapters", chapters.len() - 10);
                        }
                    }
                }
                DetailsState::Error(e) => println!("Error loading details: {}", e),
                _ => println!("Details state: {:?}", state.details_state),
            }
        }
        Commands::Read { plugin_id, chapter_id } => {
            println!("\n=== READING MANGA CHAPTER: {} ===", chapter_id);
            engine
                .store
                .dispatch(Intent::ReadChapter {
                    plugin_id: PluginId(plugin_id),
                    chapter_id,
                })
                .await;
            let state = engine.store.get_state().await;

            match state.reader_state {
                ReaderState::Loaded { chapter_id, pages } => {
                    println!("Chapter {} loaded successfully with {} pages:", chapter_id, pages.len());
                    for p in pages {
                        println!("  Page {}: {}", p.page_number, p.image_url);
                    }
                }
                ReaderState::Error(e) => println!("Error loading chapter pages: {}", e),
                _ => println!("Reader state: {:?}", state.reader_state),
            }
        }
        Commands::Library { category } => {
            let cat_enum = category
                .as_ref()
                .and_then(|c| MediaCategory::from_str(c));
            println!("\n=== LOCAL SQLITE LIBRARY ({:?}) ===", cat_enum);
            engine.store.dispatch(Intent::LoadLibrary(cat_enum)).await;
            let state = engine.store.get_state().await;

            match state.library_state {
                LibraryState::Loaded(items) => {
                    if items.is_empty() {
                        println!("Library is currently empty.");
                    } else {
                        for item in items {
                            println!(
                                "  * [{}] {} - Category: {} (Plugin: {})",
                                item.id, item.title, item.category, item.plugin_id
                            );
                        }
                    }
                }
                LibraryState::Error(e) => println!("Error loading library: {}", e),
                _ => println!("Library state: {:?}", state.library_state),
            }
        }
        Commands::LibraryAdd { plugin_id, item_id } => {
            println!("\n=== ADDING TO LIBRARY: {} (Plugin: {}) ===", item_id, plugin_id);
            // First fetch details to get the MediaItem
            engine
                .store
                .dispatch(Intent::GetDetails {
                    plugin_id: PluginId(plugin_id.clone()),
                    item_id: item_id.clone(),
                })
                .await;
            let state = engine.store.get_state().await;

            match state.details_state {
                DetailsState::Loaded { item, .. } => {
                    engine.store.dispatch(Intent::AddToLibrary(item.clone())).await;
                    println!("Successfully added \"{}\" to local SQLite library!", item.title);
                }
                DetailsState::Error(e) => println!("Failed to find item to add: {}", e),
                _ => println!("Could not load item details."),
            }
        }
    }

    Ok(())
}

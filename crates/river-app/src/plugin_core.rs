use kdl::KdlDocument;

/// `plugin-core`: The Universal Plugin Gateway.
///
/// This module handles plugins as a whole.
/// Its simple function is to read declarative KDL layout documents (from disk, memory, or network)
/// and validate them before handing them off to specialized engines like `plugin-ui-core`.
pub fn load_kdl_plugin(raw_kdl: &str) -> Result<KdlDocument, String> {
    raw_kdl
        .parse::<KdlDocument>()
        .map_err(|e| format!("KDL Parse Error: {}", e))
}

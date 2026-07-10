//! Data-driven background framework.
//!
//! Replaces the old hardcoded `match background_type` implementation.
//! Plugin authors write KDL describing layered shapes whose geometry and color
//! are driven by math expressions.

pub mod bg_color;
pub mod bg_render;
pub mod bg_spec;
pub mod expr;

pub use bg_color::{lerp_color};
pub use bg_render::{draw_kdl_background, render_background, BackgroundCache};
pub use bg_spec::{parse_background_kdl, BackgroundSpec, SpecError};

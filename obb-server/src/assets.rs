// SPDX-License-Identifier: GPL-3.0-or-later
//! The extension's static files, compiled into the binary so one executable is the whole install.

pub const MANIFEST_JSON: &str = include_str!("../assets/manifest.json");
pub const ICON_SVG: &str = include_str!("../assets/icon.svg");
pub const INDEX_HTML: &str = include_str!("../assets/index.html");
pub const OBR_BRIDGE_JS: &str = include_str!("../assets/obr_bridge.js");

/// `(file name, content type, contents)` for every asset.
pub const ALL: [(&str, &str, &str); 4] = [
    (
        "manifest.json",
        "application/json; charset=utf-8",
        MANIFEST_JSON,
    ),
    ("icon.svg", "image/svg+xml", ICON_SVG),
    ("index.html", "text/html; charset=utf-8", INDEX_HTML),
    (
        "obr_bridge.js",
        "text/javascript; charset=utf-8",
        OBR_BRIDGE_JS,
    ),
];

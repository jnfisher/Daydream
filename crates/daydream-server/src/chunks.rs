//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "World-VUFH5AGU.js" => include_str!("chunks/World-VUFH5AGU.js"),
        "chunk-EELWAIY6.js" => include_str!("chunks/chunk-EELWAIY6.js"),
        _ => return None,
    })
}

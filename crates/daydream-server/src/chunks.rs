//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "World-QD5IJHOA.js" => include_str!("chunks/World-QD5IJHOA.js"),
        "chunk-I7OR3MLQ.js" => include_str!("chunks/chunk-I7OR3MLQ.js"),
        _ => return None,
    })
}

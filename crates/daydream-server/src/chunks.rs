//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Library-H5XRAWJM.js" => include_str!("chunks/Library-H5XRAWJM.js"),
        "World-XOK6JKP3.js" => include_str!("chunks/World-XOK6JKP3.js"),
        "chunk-BUXFAU4A.js" => include_str!("chunks/chunk-BUXFAU4A.js"),
        "chunk-HD3BSVQF.js" => include_str!("chunks/chunk-HD3BSVQF.js"),
        "chunk-VA475VRY.js" => include_str!("chunks/chunk-VA475VRY.js"),
        "chunk-ZVRKVWLB.js" => include_str!("chunks/chunk-ZVRKVWLB.js"),
        _ => return None,
    })
}

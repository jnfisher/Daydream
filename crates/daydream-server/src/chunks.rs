//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Library-4W6AWDEL.js" => include_str!("chunks/Library-4W6AWDEL.js"),
        "World-LSCTILBH.js" => include_str!("chunks/World-LSCTILBH.js"),
        "chunk-BHMCH2WR.js" => include_str!("chunks/chunk-BHMCH2WR.js"),
        "chunk-CQBO3DB3.js" => include_str!("chunks/chunk-CQBO3DB3.js"),
        "chunk-JWUG5ASI.js" => include_str!("chunks/chunk-JWUG5ASI.js"),
        "chunk-KPH5DVEF.js" => include_str!("chunks/chunk-KPH5DVEF.js"),
        "chunk-UM33N6EX.js" => include_str!("chunks/chunk-UM33N6EX.js"),
        "chunk-WHOMC6B3.js" => include_str!("chunks/chunk-WHOMC6B3.js"),
        "chunk-WYCZXW2X.js" => include_str!("chunks/chunk-WYCZXW2X.js"),
        "chunk-ZINETL6G.js" => include_str!("chunks/chunk-ZINETL6G.js"),
        "parts-IANIWENJ.js" => include_str!("chunks/parts-IANIWENJ.js"),
        _ => return None,
    })
}

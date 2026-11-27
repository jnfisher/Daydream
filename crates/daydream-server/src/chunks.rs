//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Library-AX2NO2OL.js" => include_str!("chunks/Library-AX2NO2OL.js"),
        "World-LSCTILBH.js" => include_str!("chunks/World-LSCTILBH.js"),
        "chunk-BHMCH2WR.js" => include_str!("chunks/chunk-BHMCH2WR.js"),
        "chunk-IH35STIX.js" => include_str!("chunks/chunk-IH35STIX.js"),
        "chunk-KBCL33AO.js" => include_str!("chunks/chunk-KBCL33AO.js"),
        "chunk-KPH5DVEF.js" => include_str!("chunks/chunk-KPH5DVEF.js"),
        "chunk-R5ALQEIJ.js" => include_str!("chunks/chunk-R5ALQEIJ.js"),
        "chunk-UM33N6EX.js" => include_str!("chunks/chunk-UM33N6EX.js"),
        "chunk-WYCZXW2X.js" => include_str!("chunks/chunk-WYCZXW2X.js"),
        "chunk-ZINETL6G.js" => include_str!("chunks/chunk-ZINETL6G.js"),
        "parts-G5KLVOAI.js" => include_str!("chunks/parts-G5KLVOAI.js"),
        _ => return None,
    })
}

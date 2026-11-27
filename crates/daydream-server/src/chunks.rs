//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Library-II45Z5GH.js" => include_str!("chunks/Library-II45Z5GH.js"),
        "World-ZTJ7GS63.js" => include_str!("chunks/World-ZTJ7GS63.js"),
        "chunk-2GNIB3DY.js" => include_str!("chunks/chunk-2GNIB3DY.js"),
        "chunk-4QYZIXSO.js" => include_str!("chunks/chunk-4QYZIXSO.js"),
        "chunk-BFRLSUJZ.js" => include_str!("chunks/chunk-BFRLSUJZ.js"),
        "chunk-EUXJJYII.js" => include_str!("chunks/chunk-EUXJJYII.js"),
        "chunk-KJY54EVY.js" => include_str!("chunks/chunk-KJY54EVY.js"),
        "chunk-L7TO5KE7.js" => include_str!("chunks/chunk-L7TO5KE7.js"),
        "chunk-MGXZ74XK.js" => include_str!("chunks/chunk-MGXZ74XK.js"),
        "chunk-OPNA42NK.js" => include_str!("chunks/chunk-OPNA42NK.js"),
        "parts-S7FWELVK.js" => include_str!("chunks/parts-S7FWELVK.js"),
        _ => return None,
    })
}

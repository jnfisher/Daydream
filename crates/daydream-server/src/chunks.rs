//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Grid-EHIJ7WTK.js" => include_str!("chunks/Grid-EHIJ7WTK.js"),
        "Library-4772IHLK.js" => include_str!("chunks/Library-4772IHLK.js"),
        "World-UQCAZRN6.js" => include_str!("chunks/World-UQCAZRN6.js"),
        "chunk-4HQ4CMVM.js" => include_str!("chunks/chunk-4HQ4CMVM.js"),
        "chunk-C2RVLTJG.js" => include_str!("chunks/chunk-C2RVLTJG.js"),
        "chunk-EFIQIKOR.js" => include_str!("chunks/chunk-EFIQIKOR.js"),
        "chunk-G7LNNTT2.js" => include_str!("chunks/chunk-G7LNNTT2.js"),
        "chunk-G7RXW27E.js" => include_str!("chunks/chunk-G7RXW27E.js"),
        "chunk-KUBBZR54.js" => include_str!("chunks/chunk-KUBBZR54.js"),
        "chunk-NDCY5STL.js" => include_str!("chunks/chunk-NDCY5STL.js"),
        "chunk-NDKXFK4B.js" => include_str!("chunks/chunk-NDKXFK4B.js"),
        "chunk-RZG5H7DT.js" => include_str!("chunks/chunk-RZG5H7DT.js"),
        "chunk-SD5X2VAL.js" => include_str!("chunks/chunk-SD5X2VAL.js"),
        "chunk-VSSQKPNX.js" => include_str!("chunks/chunk-VSSQKPNX.js"),
        "chunk-YLMHBJAB.js" => include_str!("chunks/chunk-YLMHBJAB.js"),
        "parts-IGWOFJDM.js" => include_str!("chunks/parts-IGWOFJDM.js"),
        _ => return None,
    })
}

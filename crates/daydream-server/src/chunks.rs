//! The UI's code split from app.js by `npm run build` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "Grid-NPVBDGGO.js" => include_str!("chunks/Grid-NPVBDGGO.js"),
        "Library-CLXZCP74.js" => include_str!("chunks/Library-CLXZCP74.js"),
        "Story-UWQWJSEE.js" => include_str!("chunks/Story-UWQWJSEE.js"),
        "World-A7IKYVKB.js" => include_str!("chunks/World-A7IKYVKB.js"),
        "chunk-2G5IFPEC.js" => include_str!("chunks/chunk-2G5IFPEC.js"),
        "chunk-33EECMPF.js" => include_str!("chunks/chunk-33EECMPF.js"),
        "chunk-37OPCN7H.js" => include_str!("chunks/chunk-37OPCN7H.js"),
        "chunk-545BM4AG.js" => include_str!("chunks/chunk-545BM4AG.js"),
        "chunk-DX2S3ZEQ.js" => include_str!("chunks/chunk-DX2S3ZEQ.js"),
        "chunk-EDDDVOCX.js" => include_str!("chunks/chunk-EDDDVOCX.js"),
        "chunk-EOP3BVNQ.js" => include_str!("chunks/chunk-EOP3BVNQ.js"),
        "chunk-FSZA3WIN.js" => include_str!("chunks/chunk-FSZA3WIN.js"),
        "chunk-FXD2FWJR.js" => include_str!("chunks/chunk-FXD2FWJR.js"),
        "chunk-G7QJBB6B.js" => include_str!("chunks/chunk-G7QJBB6B.js"),
        "chunk-HUQQPHGG.js" => include_str!("chunks/chunk-HUQQPHGG.js"),
        "chunk-I7522FPL.js" => include_str!("chunks/chunk-I7522FPL.js"),
        "chunk-JMY2ZPK3.js" => include_str!("chunks/chunk-JMY2ZPK3.js"),
        "chunk-JXUCZ4RQ.js" => include_str!("chunks/chunk-JXUCZ4RQ.js"),
        "chunk-QGD6QFVH.js" => include_str!("chunks/chunk-QGD6QFVH.js"),
        "chunk-R7WAV2RS.js" => include_str!("chunks/chunk-R7WAV2RS.js"),
        "chunk-S5WC37NH.js" => include_str!("chunks/chunk-S5WC37NH.js"),
        "chunk-S7545Z6Q.js" => include_str!("chunks/chunk-S7545Z6Q.js"),
        "chunk-UFUYQ4XM.js" => include_str!("chunks/chunk-UFUYQ4XM.js"),
        "chunk-VCJZGHVG.js" => include_str!("chunks/chunk-VCJZGHVG.js"),
        "chunk-WCGFBHIF.js" => include_str!("chunks/chunk-WCGFBHIF.js"),
        "chunk-ZI5IBFR4.js" => include_str!("chunks/chunk-ZI5IBFR4.js"),
        "chunk-ZQZWUS5I.js" => include_str!("chunks/chunk-ZQZWUS5I.js"),
        "parts-VW7LPOLD.js" => include_str!("chunks/parts-VW7LPOLD.js"),
        _ => return None,
    })
}

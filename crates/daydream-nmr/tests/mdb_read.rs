//! Reading a `.mdb`: a Jet 4 file written by datacraft-jet's own writer (no indexes; enough
//! to check the reader keeps NMR's values).

use datacraft_jet::StorageValue as S;
use datacraft_jet::jet::{Jet4Writer, JetColumn, JetType, create::create_jet4_file};
use daydream_nmr::{Value, mdb};

#[test]
fn reads_nmr_tables_and_ignores_others() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.mdb");
    create_jet4_file(&path, 0.0).unwrap();
    let mut w = Jet4Writer::new(std::fs::read(&path).unwrap()).unwrap();
    let mut text = JetColumn::new("Line 1", JetType::Text, 0x02, 1, 0, 148);
    text.var_index = Some(0);
    let messages = w
        .create_table(
            "Messages",
            &[
                JetColumn::new("Number", JetType::LongInt, 0x03, 0, 0, 4),
                text,
            ],
        )
        .unwrap();
    for (n, s) in [(1, "You hit\0\u{20AC}x"), (-5, "")] {
        w.insert_row(messages, &[S::Int(n), S::Text(s.into())])
            .unwrap();
    }
    let other = w
        .create_table(
            "Lairs",
            &[JetColumn::new("N", JetType::Double, 0x03, 0, 0, 8)],
        )
        .unwrap();
    w.insert_row(other, &[S::Float(1.5)]).unwrap();
    std::fs::write(&path, w.into_file()).unwrap();

    let data = mdb::read(&path).unwrap();
    assert_eq!(data.tables.len(), 1);
    let t = data.table("Messages").unwrap();
    assert_eq!(t.columns, ["Number", "Line 1"]);
    assert_eq!(
        t.rows,
        [
            vec![Value::Long(1), Value::Text("You hit\0\u{20AC}x".into())],
            vec![Value::Long(-5), Value::Text(String::new())],
        ]
    );
}

//! Interaction events are NEVER read by the engine (S2 §7.6): no module other than the writer
//! mentions the folder or the module. Determinism of `rank` is untouched by construction.
#[test]
fn no_engine_module_but_the_writer_references_ui_events() {
    for entry in std::fs::read_dir("src").unwrap().flatten() {
        let p = entry.path();
        if p.extension().map(|e| e == "rs").unwrap_or(false)
            && p.file_name().unwrap() != "uievents.rs"
            && p.file_name().unwrap() != "lib.rs"
            // controller ruling R-P1: history is the console-only transport that stages the folder; it never reads a record
            && p.file_name().unwrap() != "history.rs"
        {
            let text = std::fs::read_to_string(&p).unwrap();
            assert!(!text.contains("events-ui") && !text.contains("uievents::"), "{} references interaction events", p.display());
        }
    }
}

# F11c2 shots: step 10's hours Edit

Headless Edge, 1280x900, a scratch copy of `app/static` with a stub `__TAURI__` that answers `your_week`,
`commitment_proposals`, `account_status` and `set_fields`; invented data (two classes, one with two meets
entries), no real student data. The prototype is the same `o9-your-week` screen as in F11p3's shots (the
prototype shows Edit as a button only and has no hours control, so the prototype column is the screen the
Edit opens from).

| Shot | Built | Prototype |
|---|---|---|
| Edit open on a class with two meets entries, dark | f11c2-edit-dark-built.png | f11c2-edit-dark-prototype.png |
| the same, light | f11c2-edit-light-built.png | f11c2-edit-light-prototype.png |
| Save refused, dark (the stub's `set_fields` answers `{ok: false, error: ...}`; the page shows it as given) | f11c2-edit-refusal-dark-built.png | none |
| Save refused, light | f11c2-edit-refusal-light-built.png | none |

The refusal text in the refusal shots is a placeholder from the stub, not the engine's sentence; the page holds no
hours sentence of its own.

Each entry is a group ("Time 1", "Time 2") with Mon..Sun toggles (pressed = a day), labelled start and end times
("Time 1 start") and Remove; "Add another time" follows the last entry. With one entry the labels read "start" / "end".

The built shots are 1280x900, not the 1280x800 of the design's section 14.58.7: an open Edit with two entries is
taller than 800 px and would crop the Save row and the second entry.

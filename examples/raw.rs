extern crate nixterm;

use nixterm::events::Key;
use nixterm::Term;

pub fn main() {
    let term = Term::new().unwrap();

    // save the terminal's settings
    let settings = term.settings();

    term.print("Entering raw mode...\n").unwrap();

    // Clone the settings an add the necessary options for raw mode
    // stop & start output can be confusing (usually they are Ctrl-S & Ctrl-Q) so disable them.
    term.update(settings.clone().raw().stop_output('\0').start_output('\0'))
        .unwrap();
    term.print("Try pressing a few keys keys (Ctrl-C to quit): ")
        .unwrap();

    for key in term
        .read_keys()
        .map(Result::unwrap)
        .take_while(|k| k != &Key::Control('C'))
    {
        term.save_cursor();
        term.clear_line_after_cursor();
        term.print(format!("{:?}", key)).unwrap();
        term.flush();
        term.restore_cursor();
    }

    // In raw mode '\n' will do nothing but move the cursor down a line,
    // so the '\r' is necessary to move the cursor back to the start.
    term.print("\n\r").unwrap();

    // Revert back to the settings saved before entering raw mode
    term.update(settings).unwrap();
}

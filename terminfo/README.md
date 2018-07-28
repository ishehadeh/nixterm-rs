# TermInfo

TermInfo is a rust library for reading terminfo files.

## Usage

The two structures that you will use most in terminfo are `TermInfo` and `TermInfoBuf`.
These structures are similar to the `str` -> `String` dynamic. `TermInfo` is a view into a `[u8]` buffer, which
may be allocated anywhere. `TermInfoBuf` contains the same data, but it owns a copy of it on the heap.

### Checking Properties

Getting properties from the running terminal is easy!

```rust
extern crate terminfo;

fn main() {
    let info = terminfo::from_env().unwrap();

    print!("Terminal: {}", info.names[0]);
    if info.names.len() > 1 {
        print!(" (AKA \"{}\")", info.names[1..].join("\", \""));
    }
    println!();

    println!(
        "Does this terminal support automatic margins? {}",
        info.boolean(terminfo::AutoRightMargin)
    );
    println!(
        "How many colors does this terminal support? {}",
        info.number(terminfo::MaxColors).unwrap()
    );
    println!(
        "How does this terminal represent the \"F10\" Key? {:?}",
        info.string(terminfo::KeyF10).unwrap()
    );
}
```

First, the main function creates a new `TermInfoBuf` with the `from_env` function. This function uses the environment (specifically the `$TERM` variable)
to find the current terminal's terminfo file, and then it parses it.

Next, we get the primary name of the terminal, and print it (the `TermInfoBuf::name` method is basically `TermInfoBuf::names()[0]`).
If the terminal has any extra names we print those too.

Finally, just to show off this program prints a few of the terminal's capabilites, using the `boolean`, `number` and `string` methods.
These three methods, `boolean`, `number`, and `string`, each fetch a value, denoted by a their first argument.
The type of the argument passed to each of these functions is different, `boolean` is passed a `BooleanField`, `number` a `NumericField` and `string` a `StringField`.
For convenience all these enums' values have been imported into the `terminfo` namespace.
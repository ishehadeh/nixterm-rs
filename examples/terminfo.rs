extern crate nixterm;

use nixterm::terminfo;

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

    println!(
        "How do a move to <0, 0>? {:?}",
        info.exec(terminfo::CursorAddress)
            .unwrap()
            .arg(0)
            .arg(0)
            .string()
            .unwrap()
    );
}

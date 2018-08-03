extern crate nixterm;

use nixterm::term::Term;

pub fn main() {
    let term = Term::new().unwrap();

    // save the terminal's settings
    let settings = term.settings();

    term.print("_Demo Signup Form_\n\n").unwrap();

    let name = term.prompt("[+fg:green] -->[-fg] Username: ").unwrap();

    // Turn echo off to hide the password
    term.update(settings.clone().echo(false)).unwrap();

    let password = term.prompt("[+fg:green] -->[-fg] Password: ").unwrap();

    term.print("\n").unwrap();

    let password2 = term.prompt("[+fg:green] -->[-fg] Password (Confirm): ")
        .unwrap();

    // restore the original settings
    term.update(settings).unwrap();

    if password != password2 {
        term.print("\n_[+fg:red]Passwords don't match![-fg]_\n")
            .unwrap();
    } else {
        term.print(format!(
            "\n\n_[+fg:cyan]Thank you, {}![-fg]_\n",
            name.trim()
        )).unwrap();
    }
}

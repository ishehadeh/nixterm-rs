extern crate nixterm;

use nixterm::term::Term;

pub fn main() {
    let term = Term::new().unwrap();

    // save the terminal's settings
    let settings = term.settings();

    term.writer()
        .bold()
        .println("Demo Signup Form")
        .println("")
        .done()
        .unwrap();

    term.writer()
        .foreground("green")
        .print("\t--> ")
        .print("Username: ")
        .done()
        .unwrap();

    let name = term.readline().unwrap();

    // Turn echo off to hide the password
    term.update(settings.clone().echo(false)).unwrap();

    term.writer()
        .foreground("green")
        .print("\t--> ")
        .print("Password: ")
        .done()
        .unwrap();
    let password = term.readline().unwrap();

    term.println("");

    term.writer()
        .foreground("green")
        .print("\t--> ")
        .print("Password (Confirm): ")
        .done()
        .unwrap();
    let password2 = term.readline().unwrap();

    // restore the original settings
    term.update(settings).unwrap();

    term.println("");
    term.println("");

    if password != password2 {
        term.writer()
            .foreground("red")
            .bold()
            .println("Passwords don't match!")
            .done()
            .unwrap();
    } else {
        term.writer()
            .foreground("cyan")
            .print("Thank you, ")
            .foreground("cyan")
            .bold()
            .println(name.trim())
            .done()
            .unwrap();
    }
}

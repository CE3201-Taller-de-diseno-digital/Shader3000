use gtk::prelude::*;

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    sourceview::View::static_type();

    let glade_src = include_str!("IDE.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::Window = builder.get_object("main_window").unwrap();

    window.show_all();

    gtk::main();
}

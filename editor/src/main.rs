extern crate gio;
extern crate glib;
extern crate gtk;

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use sourceview::*;
use std::env::args;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

fn main() {
    let application = gtk::Application::new(Some("com.editor.animationLED"), Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {

        build_ui(app);
    });

    //Start main loop
    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {

    //               ____________________
    //______________/  Create main window

    let glade_src = include_str!("IDE.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::ApplicationWindow = builder
        .get_object("main_window")
        .expect("Couldn't get window");
    window.set_application(Some(application));

    window.set_position(gtk::WindowPosition::Center);

    let provider = gtk::CssProvider::new();
    // Load the CSS file
    provider.load_from_path("editor/src/resources/style.css").unwrap();
    gtk::StyleContext::add_provider_for_screen(&window.get_screen().unwrap(),&provider,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    //               ___________________
    //______________/  Get components

    //Buttons
    let compile_run: gtk::Button = builder.get_object("comp_and_run").unwrap();
    let compile: gtk::Button = builder.get_object("comp").unwrap();

    //Menu Items
    //File
    let new: gtk::MenuItem = builder.get_object("new").unwrap();
    let open: gtk::MenuItem = builder.get_object("open").unwrap();
    let save: gtk::MenuItem = builder.get_object("save").unwrap();
    let save_as: gtk::MenuItem = builder.get_object("save_as").unwrap();
    let quit: gtk::MenuItem = builder.get_object("quit").unwrap();
    //Help
    let about: gtk::MenuItem = builder.get_object("about").unwrap();
    let about_win : gtk::AboutDialog = builder.get_object("about_win").unwrap();

    //Scrolled Window
    let scroll: gtk::ScrolledWindow = builder.get_object("sourceHold").unwrap();

    //SourceView
    let buffer = sourceview::Buffer::new_with_language(
        &sourceview::LanguageManager::get_default()
            .unwrap()
            .get_language("c")
            .unwrap(),
    );
    buffer.set_highlight_syntax(true);

    let sourceview = sourceview::View::new_with_buffer(&buffer);
    sourceview.set_auto_indent(true);
    sourceview.set_indent_on_tab(true);
    sourceview.set_show_line_numbers(true);
    sourceview.set_smart_backspace(true);

    scroll.add(&sourceview);

    //Themes
    let themes: sourceview::StyleSchemeChooserButton = builder.get_object("themes").unwrap();

    //Terminal
    let terminal: gtk::TextView = builder.get_object("terminal").unwrap();
    terminal.set_widget_name("terminal");

    //Notebook
    let doc_name: gtk::Label = builder.get_object("doc_name").unwrap();

    //File
    let current_file = gtk::Label::new(Some("unnamed.txt"));


    //               ___________________
    //______________/  Add funtionality

    compile_run.connect_clicked(clone!(@weak window, @weak save => move |_| {

       save.activate();

    }));

    compile.connect_clicked(clone!(@weak window , @weak save=> move |_| {

        save.activate();

    }));

    new.connect_activate(clone!(@weak sourceview , @weak doc_name, @weak current_file, @weak save => move |_| {

            save.activate();

            sourceview
                .get_buffer()
                .expect("Couldn't get window")
                .set_text("");

            doc_name.set_text("unnamed");

            current_file.set_text("unnamed.txt");

        }),
    );

    let sourceview2 = sourceview.clone();
    let doc_name2 = doc_name.clone();
    let current_file2 = current_file.clone();

    open.connect_activate(clone!(@weak window , @weak save => move |_| {

        let file_chooser = gtk::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk::FileChooserAction::Open,
        );
        file_chooser.add_buttons(&[
            ("Open", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ]);

        let save_for_chooser = save.clone();

        file_chooser.connect_response(clone!(@weak sourceview2, @weak doc_name2 ,@weak current_file2 ,@weak save_for_chooser=> move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

                save_for_chooser.activate();

                let filename = file_chooser.get_filename().expect("Couldn't get filename");
                let file = File::open(&filename).expect("Couldn't open file");

                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                let _ = reader.read_to_string(&mut contents);

                sourceview2
                    .get_buffer()
                    .expect("Couldn't get window")
                    .set_text(&contents);

                match filename.to_str() {
                    None => panic!("new path is not a valid UTF-8 sequence"),
                    Some(name) => {  let chunks:Vec<&str> = name.split("/").collect();
                                     doc_name2.set_text(&chunks[chunks.len()-1]);
                                     current_file2.set_text(name);
                                    }
                }
            }
            file_chooser.close();
        }));

        file_chooser.show_all();
    }));

    let current_file4 = current_file.clone();

    let sourceview4 = sourceview.clone();

    save.connect_activate(clone!(@weak current_file4,@weak sourceview4 => move |_| {

        let filename = current_file4.get_text();

        let buffer = sourceview4.get_buffer().expect("Couldn't get window");

        let bounds = buffer.get_bounds();

        let text = buffer.get_text(&bounds.0,&bounds.1,true);

        let path = Path::new(filename.as_str());
        let display = path.display();

        let mut file = match File::create(&path) {
                Err(why) => panic!("couldn't create {}: {}", display, why),
                Ok(file) => file,
            };

        match file.write_all(text.unwrap().as_str().as_bytes()) {
                Err(why) => panic!("couldn't write to {}: {}", display, why),
                Ok(_) => println!("successfully wrote to {}", display),
            };

    }));

    let sourceview3 = sourceview.clone();

    let doc_name3 = doc_name.clone();

    let current_file3 = current_file.clone();

    save_as.connect_activate(clone!(@weak window => move |_| {

        let file_chooser = gtk::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk::FileChooserAction::Save,
        );
        file_chooser.add_buttons(&[
            ("Save", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ]);

        file_chooser.set_do_overwrite_confirmation(true);

        file_chooser.connect_response(clone!(@weak sourceview3, @weak doc_name3 ,@weak current_file3 => move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

                let filename = file_chooser.get_filename().expect("Couldn't get filename");

                let buffer = sourceview3.get_buffer().expect("Couldn't get window");

                let bounds = buffer.get_bounds();

                let text = buffer.get_text(&bounds.0,&bounds.1,true);

                let path = Path::new(filename.to_str().unwrap());

                let display = path.display();

                let mut file = match File::create(&path) {
                    Err(why) => panic!("couldn't create {}: {}", display, why),
                    Ok(file) => file,
                };

                match file.write_all(text.unwrap().as_str().as_bytes()) {
                    Err(why) => panic!("couldn't write to {}: {}", display, why),
                    Ok(_) => println!("successfully wrote to {}", display),
                }

                match filename.to_str() {
                    None => panic!("new path is not a valid UTF-8 sequence"),
                    Some(name) => {  let chunks:Vec<&str> = name.split("/").collect();
                                     doc_name3.set_text(&chunks[chunks.len()-1]);
                                     current_file3.set_text(name);
                                    }
                }

            }

            file_chooser.close();
        }));

        file_chooser.show_all();
    }));

    themes.connect_property_style_scheme_notify(clone!(@weak buffer, @weak themes => move |_| {

        let scheme = themes.get_style_scheme();
        buffer.set_style_scheme(Some(&scheme).unwrap().as_ref());

    }));

    about.connect_activate(move |_| {

        about_win.show_all();

    });

    quit.connect_activate(clone!(@weak window => move |_| {

        window.close();

    }));

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
}

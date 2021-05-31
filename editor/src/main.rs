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
use std::rc::Rc;

fn main() {
    let application = gtk::Application::new(Some("com.editor.animationLED"), Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    //Start main loop
    application.run(&args().collect::<Vec<_>>());
}

fn append_text_column(tree: &gtk::TreeView) {
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();
    column.set_title("Project Folder");

    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);
    tree.append_column(&column);
}

fn build_ui(application: &gtk::Application) {
    //sourceview::View::static_type();

    //               ____________________
    //______________/  Create main window

    let glade_src = include_str!("IDE.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::ApplicationWindow = builder
        .get_object("main_window")
        .expect("Couldn't get window1");
    window.set_application(Some(application));

    //               ___________________
    //______________/  Get components

    //Buttons
    let compile_run: gtk::Button = builder.get_object("comp_and_run").unwrap();
    let compile: gtk::Button = builder.get_object("comp").unwrap();

    //Menu Items
    //File
    let new: gtk::MenuItem = builder.get_object("new").unwrap();
    let open: gtk::MenuItem = builder.get_object("open").unwrap();
    let open_folder: gtk::MenuItem = builder.get_object("open_folder").unwrap();
    let save: gtk::MenuItem = builder.get_object("save").unwrap();
    let save_as: gtk::MenuItem = builder.get_object("save_as").unwrap();
    let quit: gtk::MenuItem = builder.get_object("quit").unwrap();
    //Edit
    let cut: gtk::MenuItem = builder.get_object("cut").unwrap();
    let copy: gtk::MenuItem = builder.get_object("copy").unwrap();
    let paste: gtk::MenuItem = builder.get_object("paste").unwrap();
    let delete: gtk::MenuItem = builder.get_object("delete").unwrap();
    //Theme
    //let bright :   gtk::MenuItem = builder.get_object("bright").unwrap();
    //let dark :     gtk::MenuItem = builder.get_object("dark").unwrap();
    //Help
    let about: gtk::MenuItem = builder.get_object("about").unwrap();

    //Scrolled Window

    let scroll: gtk::ScrolledWindow = builder.get_object("sourceHold").unwrap();

    //SourceView
    //let sourceview: sourceview::View = builder.get_object("source").unwrap();

    //let buffer : sourceview::Buffer = builder.get_object("textbuffer1").unwrap();

    let buffer = sourceview::Buffer::new_with_language(
        &sourceview::LanguageManager::get_default()
            .unwrap()
            .get_language("c")
            .unwrap(),
    );

    //let buffer = sourceview.get_buffer().unwrap() as sourceview::Buffer;

    buffer.set_highlight_syntax(true);

    let sourceview = sourceview::View::new_with_buffer(&buffer);

    scroll.add(&sourceview);

    //print!("{:?}",sourceview::LanguageManager::get_default().unwrap().get_language_ids());

    //Notebook
    let doc_name: gtk::Label = builder.get_object("doc_name").unwrap();

    //TreeView
    let left_tree: gtk::TreeView = builder.get_object("treeview").unwrap();
    let store = gtk::TreeStore::new(&[String::static_type()]);
    left_tree.set_model(Some(&store));
    //Create treeview elements
    left_tree.set_model(Some(&store));
    left_tree.set_headers_visible(true);
    append_text_column(&left_tree);

    //File
    //let mut current_file = "none";
    let current_file = gtk::Label::new(Some("unnamed.txt"));
    //let mut current_file = Rc::new(current_file);

    //               ___________________
    //______________/  Add funtionality

    //TODO:comp_run
    //TODO:comp

    //let store = Rc::new(store);
    //let doc_name = Rc::new(doc_name);

    new.connect_activate(
        clone!(@weak sourceview , @weak doc_name, @weak current_file => move |_| {

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

    open.connect_activate(clone!(@weak window => move |_| {

        let file_chooser = gtk::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk::FileChooserAction::Open,
        );
        file_chooser.add_buttons(&[
            ("Open", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ]);
        file_chooser.connect_response(clone!(@weak sourceview2, @weak doc_name2 ,@weak current_file2=> move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

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

    //let store = Rc::new(store);

    open_folder.connect_activate(clone!(@weak window => move |_| {

        //let store = Rc::clone(&store);

        let folder_chooser = gtk::FileChooserDialog::new(
            Some("Choose a file"), 
            Some(&window),
            gtk::FileChooserAction::SelectFolder);

        folder_chooser.add_buttons(&[
            ("Open", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel)
        ]);

        folder_chooser.set_select_multiple(true);

        folder_chooser.connect_response(clone!(@weak store => move|folder_chooser, response| {
            if response == gtk::ResponseType::Ok {

                store.clear();

                folder_chooser.select_all();

                let folder = folder_chooser.get_filenames();

                //let files = folder.enumerate_children_async();

                for filename in folder{

                    //store.insert_with_values(None, None, &[0], &[&"HELO"]);

                    match filename.to_str() {
                        None => panic!("new path is not a valid UTF-8 sequence"),
                        Some(name) => {  let chunks:Vec<&str> = name.split("/").collect();
                                        store.insert_with_values(None, None, &[0], &[&format!("{}",&chunks[chunks.len()-1])]);
                                        }
                    }
                }

            }
            folder_chooser.close();
        }));
        folder_chooser.show_all();
    }));

    let current_file4 = current_file.clone();

    let sourceview4 = sourceview.clone();

    //let buffer2 = buffer.clone();

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

    quit.connect_activate(clone!(@weak window => move |_| {
        window.close();
    }));

    // for i in 0..10 {
    //insert_with_values takes two slices: column indices and ToValue
    //trait objects. ToValue is implemented for strings, numeric types,
    //bool and Object descendants
    //   let iter = left_store.insert_with_values(None, None, &[0], &[&format!("Hello {}", i)]);

    //   for _ in 0..i {
    //     left_store.insert_with_values(Some(&iter), None, &[0], &[&"I'm a child node"]);
    //   }
    //}

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
}

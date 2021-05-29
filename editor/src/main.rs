extern crate gio;
extern crate glib;
extern crate gtk;

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;

use std::env::args;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
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

    //SourceView
    let sourceview: sourceview::View = builder.get_object("source").unwrap();

    //let source : sourceview::Buffer = sourceview.get_buffer().unwrap();

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

    //               ___________________
    //______________/  Add funtionality

    //TODO:comp_run
    //TODO:comp

    let store = Rc::new(store);
    let doc_name = Rc::new(doc_name);

    new.connect_activate(clone!(@weak sourceview => move |_| {

        sourceview
            .get_buffer()
            .expect("Couldn't get window")
            .set_text("");

    }));

    new.connect_activate(clone!(@weak doc_name => move |_| {

        doc_name.set_text("Unnamed");

    }));

    open.connect_activate(clone!(@weak window => move |_| {

        let doc_name = Rc::clone(&doc_name);

        let file_chooser = gtk::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk::FileChooserAction::Open,
        );
        file_chooser.add_buttons(&[
            ("Open", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ]);
        file_chooser.connect_response(clone!(@weak sourceview => move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

                let filename = file_chooser.get_filename().expect("Couldn't get filename");
                let file = File::open(&filename).expect("Couldn't open file");

                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                let _ = reader.read_to_string(&mut contents);

                sourceview
                    .get_buffer()
                    .expect("Couldn't get window")
                    .set_text(&contents);


                match filename.to_str() {
                    None => panic!("new path is not a valid UTF-8 sequence"),
                    Some(name) => {  let chunks:Vec<&str> = name.split("/").collect();
                                     doc_name.set_text(&chunks[chunks.len()-1]);
                                    }
                }
            }
            file_chooser.close();
        }));

        file_chooser.show_all();
    }));

    let store = Rc::new(store);

    open_folder.connect_activate(clone!(@weak window => move |_| {

        let store = Rc::clone(&store);

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

    save.connect_activate(clone!(@weak window => move |_| {

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

        file_chooser.connect_response(|file_chooser, response| {
            if response == gtk::ResponseType::Ok {


                //let filename = file_chooser.get_filename().expect("Couldn't get filename");
                //let mut file = File::create(&filename).expect("create open file");

                // Write the `LOREM_IPSUM` string to `file`, returns `io::Result<()>`
                //file.write_all(sourceview.get_buffer().unwrap().get_slice());

            }

            file_chooser.close();
        });

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

//! Editor de código
//!
//! Esta es una implementación sencilla de un editor
//! de código especializado para el lenguaje creado,
//! utilizando crates que adaptan GTK y su sourceview
//! para rust.
//!  
//! El editor cuenta con funcionalidades de lectura,
//! apertura y creacion de archivos, guardado automático
//! (a la hora de cerrar un archivo o compilarlo),
//! guardado manual y sobrescritura de archivos,
//! syntax highlight para código del lenguaje
//! creado , compilación y ejecución automáticas
//! y una terminal para desplegar errores de
//! compilación y otra información revelante.
//!
//! Además cuenta con diferentes estilos para
//! la intefaz. Y un botón de about que muestra
//! información adicional.

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
use std::process::{Command, Stdio};

/// Función main
/// Incia la aplicación de GTK
/// Llama a la función principal build_ui
/// Inicia el ciclo principal del programa
fn main() {
    let application = gtk::Application::new(Some("com.editor.animationLED"), Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    //Start main loop
    application.run(&args().collect::<Vec<_>>());
}

/// Función build_ui
/// Esta función se encarga de crear elementos gráficos,
/// obtener elementos graficos del archivo .glade y
/// de dar la funcionalidad respectiva a cada
/// uno de los botones disponibles.
fn build_ui(application: &gtk::Application) {
    //               ____________________
    //______________/  Create main window

    //Load .glade file
    let glade_src = include_str!("resources/IDE.glade");
    let builder = gtk::Builder::from_string(glade_src);

    //create main window
    let window: gtk::ApplicationWindow = builder
        .get_object("main_window")
        .expect("Couldn't get window");
    window.set_application(Some(application));
    window.set_position(gtk::WindowPosition::Center);

    // Load the .css file
    let provider = gtk::CssProvider::new();
    provider
        .load_from_path("resources/style.css")
        .unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &window.get_screen().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    //               ______________________
    //______________/  Get glade components

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
    let about_win: gtk::AboutDialog = builder.get_object("about_win").unwrap();

    //Scrolled Window
    let scroll: gtk::ScrolledWindow = builder.get_object("sourceHold").unwrap();

    //SourceView
    let buffer = sourceview::Buffer::new_with_language(
        &sourceview::LanguageManager::get_default()
            .unwrap()
            .get_language("led")
            .unwrap(),
    );

    //Set sourceview proprieties
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
    let current_file = gtk::Label::new(Some("tmp.led")); //Ruta de guardado para archivos unnamed

    //               ___________________
    //______________/  Add funtionality

    // Add "compile" button functionality
    //
    // Guardado automático
    // Envio de archivo al compilador
    // Despliegue de mensajes del compilador en la terminal
    compile.connect_clicked(
        clone!(@weak save, @weak current_file, @weak terminal=> move |_| {

            save.activate();

            let filename: &str = &current_file.get_text();

            let cmd = Command::new("./compiler").args(&[filename,"-o","exe","--target","esp8266"]).output().unwrap();
            let answer = std::str::from_utf8(&cmd.stderr).unwrap();

            let term_buffer = terminal.get_buffer().unwrap();
            let mut bounds = term_buffer.get_bounds();
            term_buffer.insert(&mut bounds.1,&answer);

        }),
    );

    // Add "compile and run" button functionality
    //
    // Ejecutar compile primero
    // Flasheo del código compilado
    // Despliegue de mensajes en la terminal
    compile_run.connect_clicked(clone!(@weak compile, @weak terminal => move |_| {

       compile.activate();

       let cmd = Command::new("espflash").args(&["/dev/ttyUSB0","exe"]).output().unwrap();
       let answer = std::str::from_utf8(&cmd.stderr).unwrap();

       let term_buffer = terminal.get_buffer().unwrap();
       let mut bounds = term_buffer.get_bounds();
       term_buffer.insert(&mut bounds.1,&answer);

    }));

    // Add "new" button functionality
    //
    // Guardado automático
    // Limpieza de buffer de texto
    // Cambio de ruta de guardado y nombre de archivo
    new.connect_activate(
        clone!(@weak sourceview ,@weak doc_name, @weak current_file, @weak save => move |_| {

            save.activate();

            sourceview
                .get_buffer()
                .expect("Couldn't get window")
                .set_text("");

            doc_name.set_text("unnamed");

            current_file.set_text("tmp.led");

        }),
    );

    let srcview_open = sourceview.clone();
    let doc_name_open = doc_name.clone();
    let current_file_open = current_file.clone();

    // Add "open" button functionality
    //
    // Abrir el seleccionardor de archivos
    // Guardado automático
    // Lectura de archivo y despligue en buffer de texto
    // Cambio de ruta de guardado y nombre del archivo
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

        file_chooser.connect_response(clone!(@weak srcview_open, @weak doc_name_open ,@weak current_file_open ,@weak save_for_chooser=> move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

                save_for_chooser.activate();

                let filename = file_chooser.get_filename().expect("Couldn't get filename");
                let file = File::open(&filename).expect("Couldn't open file");

                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                let _ = reader.read_to_string(&mut contents);

                srcview_open
                    .get_buffer()
                    .expect("Couldn't get window")
                    .set_text(&contents);

                match filename.to_str() {
                    None => panic!("new path is not a valid UTF-8 sequence"),
                    Some(name) => {  let chunks:Vec<&str> = name.split("/").collect();
                                     doc_name_open.set_text(&chunks[chunks.len()-1]);
                                     current_file_open.set_text(name);
                                    }
                }
            }
            file_chooser.close();
        }));

        file_chooser.show_all();
    }));

    let current_file_save = current_file.clone();
    let src_view_save = sourceview.clone();

    // Add "save" button functionality
    //
    // Tomar ruta de guardado actual
    // Tomar texto actual del buffer
    // Escribir bytes a la ruta especificada
    save.connect_activate(
        clone!(@weak current_file_save,@weak src_view_save,@weak terminal => move |_| {

            let filename = current_file_save.get_text();

            let buffer = src_view_save.get_buffer().expect("Couldn't get window");

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
                    Ok(_) => {  println!("successfully wrote to {}", display);
                                let term_buffer = terminal.get_buffer().unwrap();
                                let mut bounds = term_buffer.get_bounds();
                                let mut saved_msg: String = "Successfully saved at: ".to_owned();
                                saved_msg.push_str(&filename.as_str());
                                saved_msg.push_str("\n");
                                term_buffer.insert(&mut bounds.1,&saved_msg);
                                }
                };

        }),
    );

    let src_view_as = sourceview.clone();
    let doc_name_as = doc_name.clone();
    let current_file_as = current_file.clone();
    let terminal_as = terminal.clone();

    // Add "save as" button functionality
    //
    // Abrir seleccionador de archivos
    // Tomar ruta y nombre seleccionados
    // Tomar texto actual del buffer
    // Crear archivo con los bytes a la ruta y nombre especificados
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

        file_chooser.connect_response(clone!(@weak src_view_as, @weak doc_name_as ,@weak current_file_as, @weak terminal_as => move|file_chooser, response| {
            if response == gtk::ResponseType::Ok {

                let filename = file_chooser.get_filename().expect("Couldn't get filename");

                let buffer = src_view_as.get_buffer().expect("Couldn't get window");

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
                                     doc_name_as.set_text(&chunks[chunks.len()-1]);
                                     current_file_as.set_text(name);
                                     let term_buffer = terminal_as.get_buffer().unwrap();
                                     let mut bounds = term_buffer.get_bounds();
                                     let mut saved_msg: String = "Successfully saved at: ".to_owned();
                                     saved_msg.push_str(&name);
                                     saved_msg.push_str("\n");
                                     term_buffer.insert(&mut bounds.1,&saved_msg);
                                    }
                }

            }

            file_chooser.close();
        }));

        file_chooser.show_all();
    }));

    // Add themes button functionality
    //
    // Cambia el esquema de colores del buffer segun lo seleccionado
    themes.connect_property_style_scheme_notify(clone!(@weak buffer, @weak themes => move |_| {

        let scheme = themes.get_style_scheme();
        buffer.set_style_scheme(Some(&scheme).unwrap().as_ref());

    }));

    // Add "about" button funtionality
    //
    // Abre la ventana de about
    about.connect_activate(move |_| {
        about_win.show_all();
    });

    // Add "quit" button funtionality
    //
    // Cierra ventana
    quit.connect_activate(clone!(@weak window => move |_| {

        window.close();

    }));

    window.show_all();

    // When window destroyed
    //
    // Guarda el archivo
    // Detiene el ciclo principal de GTK
    window.connect_delete_event(move |_, _| {
        save.activate();
        gtk::main_quit();
        Inhibit(false)
    });
}

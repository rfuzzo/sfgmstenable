use std::fs::read_dir;
use std::io::Write;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};
use std::{env, fs};

use directories::UserDirs;
use egui::Color32;
use egui_notify::Toasts;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum EGmstValue {
    Bool(bool),
    Float(f32),
    Int(i32),
    UInt(u32),
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Gmst {
    pub name: String,
    pub value: EGmstValue,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub struct GmstViewModel {
    pub gmst: Gmst,
    pub is_edited: bool,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone)]
pub struct ModViewModel {
    pub path: PathBuf,
    pub name: String,
    pub enabled: bool,
}

/// Catpuccino themes
#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub enum ETheme {
    Frappe,
    Latte,
    Macchiato,
    Mocha,
}
pub fn get_theme(theme: &ETheme) -> catppuccin_egui::Theme {
    match theme {
        ETheme::Frappe => catppuccin_egui::FRAPPE,
        ETheme::Latte => catppuccin_egui::LATTE,
        ETheme::Macchiato => catppuccin_egui::MACCHIATO,
        ETheme::Mocha => catppuccin_egui::MOCHA,
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    pub theme: ETheme,

    // ui
    #[serde(skip)]
    pub toasts: Toasts,

    #[serde(skip)]
    pub mods: Option<Vec<ModViewModel>>,
    #[serde(skip)]
    pub default_gmsts: HashMap<String, EGmstValue>,
    #[serde(skip)]
    pub gmst_vms: Vec<GmstViewModel>,

    // runtime
    #[serde(skip)]
    pub search_filter: String,
    #[serde(skip)]
    pub display_edited: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut s = TemplateApp {
            theme: ETheme::Frappe,
            mods: None,
            toasts: Toasts::default(),
            default_gmsts: parse_gmsts(),
            gmst_vms: vec![],
            search_filter: "".to_owned(),
            display_edited: false,
        };

        s.gmst_vms = rebuild_vms(&s.default_gmsts);

        s
    }
}

fn rebuild_vms(default_gmsts: &HashMap<String, EGmstValue>) -> Vec<GmstViewModel> {
    let mut list: Vec<GmstViewModel> = vec![];
    for (name, value) in default_gmsts {
        list.push(GmstViewModel {
            gmst: Gmst {
                name: name.to_owned(),
                value: value.to_owned(),
            },
            is_edited: false,
        });
    }
    list.sort_by(|a, b| a.gmst.name.cmp(&b.gmst.name));

    list
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

fn parse_gmsts() -> HashMap<String, EGmstValue> {
    let mut map: HashMap<String, EGmstValue> = HashMap::default();

    let bytes = include_bytes!("Starfield_Game_Settings.txt");
    let reader = io::BufReader::new(bytes.as_slice());

    // Consumes the iterator, returns an (Optional) String
    reader.lines().for_each(|line| {
        if let Ok(str) = line {
            // parse first char
            let split: Vec<_> = str.split('=').collect();
            if split.len() == 2 {
                let name = split[0].trim();
                let value = split[1].trim();
                let first_char: char = name.chars().next().unwrap();

                match first_char {
                    'b' => {
                        // parse bool
                        if let Some(parsed) = match value {
                            "True" => Some(true),
                            "False" => Some(false),
                            _ => None,
                        } {
                            map.insert(name.to_owned(), EGmstValue::Bool(parsed));
                        }
                    }
                    'f' => {
                        // parse float
                        if let Ok(parsed) = value.parse::<f32>() {
                            map.insert(name.to_owned(), EGmstValue::Float(parsed));
                        }
                    }
                    'i' => {
                        // parse float
                        if let Ok(parsed) = value.parse::<i32>() {
                            map.insert(name.to_owned(), EGmstValue::Int(parsed));
                        }
                    }
                    'u' => {
                        // parse float
                        if let Ok(parsed) = value.parse::<u32>() {
                            map.insert(name.to_owned(), EGmstValue::UInt(parsed));
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    map
}

static ROW_WIDTH: f32 = 50_f32;
static BAT_NAME: &str = "my_gmsts";
//static BAT_NAME_MERGED: &str = "merged_gmsts";

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            theme,
            mods: mods_option,
            toasts,
            default_gmsts,
            gmst_vms,
            search_filter,
            display_edited,
        } = self;

        catppuccin_egui::set_theme(ctx, get_theme(theme));

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });

                // theme button on right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    // theme
                    theme_switch(ui, &mut self.theme);
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        if let Ok(cwd) = env::current_dir() {
            if !cwd.join("Starfield.exe").exists() {
                // then we are in the wrong dir
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Starfield GMST editor");
                    ui.hyperlink("https://github.com/rfuzzo/sfgmstenable");
                    ui.separator();

                    ui.heading(format!("⚠ This app needs to be run from the Starfield base directory!\nYou are in {}", cwd.display() ));
                });
                return;
            }
        }

        // fill ist of mods
        if mods_option.is_none() {
            *mods_option = Some(refresh_mods());
        }

        egui::SidePanel::left("left_panel_id").show(ctx, |ui| {
            // Headers
            ui.heading("GMSTs");
            // save buttons
            ui.add_enabled_ui(gmst_vms.iter().any(|p| p.is_edited), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            egui::RichText::new("🖹 Create command file")
                                .size(18.0)
                                .color(Color32::GREEN),
                        )
                        .clicked()
                    {
                        save_to_file(gmst_vms, BAT_NAME);
                        //add_command_to_ini(toasts, BAT_NAME);
                        *mods_option = Some(refresh_mods());
                    }

                    if ui
                        .button(
                            egui::RichText::new("🖹 Save to esm")
                                .size(18.0)
                                .color(Color32::GREEN),
                        )
                        .clicked()
                    {
                        // parse esm
                        todo!()
                    }
                });
            });

            ui.separator();

            // search bar
            ui.horizontal(|ui| {
                ui.label("Filter: ");
                ui.text_edit_singleline(search_filter);
                if ui.button("Clear").clicked() {
                    *search_filter = "".to_owned();
                }

                let fiter_btn_text = match display_edited {
                    false => "Show edited",
                    true => "Show all",
                };
                ui.toggle_value(display_edited, fiter_btn_text);
            });

            ui.separator();

            // main grid
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("main_grid_id")
                    .min_col_width(ROW_WIDTH)
                    .show(ui, |ui| {
                        // Values

                        for vm in gmst_vms.iter_mut() {
                            if !search_filter.is_empty()
                                && !vm
                                    .gmst
                                    .name
                                    .to_lowercase()
                                    .contains(&search_filter.to_lowercase())
                            {
                                continue;
                            }

                            if *display_edited && !vm.is_edited {
                                continue;
                            }

                            if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                vm.is_edited = !default_value.eq(&vm.gmst.value);
                            }

                            if vm.is_edited {
                                ui.visuals_mut().override_text_color = Some(egui::Color32::GREEN);
                            } else {
                                ui.visuals_mut().override_text_color = None;
                            }

                            ui.add_enabled_ui(false, |ui| {
                                ui.checkbox(&mut vm.is_edited, "Edited");
                            });

                            ui.label(vm.gmst.name.to_owned());
                            match vm.gmst.value {
                                EGmstValue::Bool(mut b) => {
                                    ui.checkbox(&mut b, "");
                                    vm.gmst.value = EGmstValue::Bool(b);
                                }
                                EGmstValue::Float(mut f) => {
                                    ui.add(egui::DragValue::new(&mut f).speed(0.1));
                                    vm.gmst.value = EGmstValue::Float(f);
                                }
                                EGmstValue::Int(mut i) => {
                                    ui.add(egui::DragValue::new(&mut i).speed(1));
                                    vm.gmst.value = EGmstValue::Int(i);
                                }
                                EGmstValue::UInt(mut u) => {
                                    ui.add(egui::DragValue::new(&mut u).speed(1));
                                    vm.gmst.value = EGmstValue::UInt(u);
                                }
                            }

                            //ui.add_enabled_ui(vm.is_edited, |ui| {
                            if vm.is_edited && ui.button("Reset").clicked() {
                                if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                    vm.gmst.value = *default_value;
                                }
                            }
                            //});

                            ui.end_row();
                        }
                    });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Starfield GMST editor");
            ui.hyperlink("https://github.com/rfuzzo/sfgmstenable");
            ui.separator();

            // main grid
            ui.label("Active mods. Change load order by reordering.");
            if let Some(mods) = mods_option {
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        *mods = refresh_mods();
                    }
                    if ui.button("Save").clicked() {
                        add_command_to_ini(
                            toasts,
                            mods.iter()
                                .filter(|p| p.enabled)
                                .map(|p| p.name.to_owned())
                                .collect::<Vec<_>>()
                                .as_slice(),
                        );
                    }
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response =
                        egui_dnd::dnd(ui, "dnd").show_vec(mods, |ui, mod_vm, handle, _dragging| {
                            ui.horizontal(|ui| {
                                let _clicked = handle
                                    .sense(egui::Sense::click())
                                    .ui(ui, |ui| {
                                        ui.label("::");
                                    })
                                    .clicked();
                                let r = ui.checkbox(&mut mod_vm.enabled, "");
                                if r.clicked() {
                                    if mod_vm.enabled {
                                        // copy file
                                        match fs::copy(&mod_vm.path, format!("./{}", mod_vm.name)) {
                                            Ok(_) => {
                                                toasts.success(format!("{} enabled", mod_vm.name));
                                            }
                                            Err(_) => {
                                                toasts.error(format!(
                                                    "failed to install {}",
                                                    mod_vm.name
                                                ));
                                            }
                                        }
                                    } else {
                                        // delete file
                                        match fs::remove_file(format!("./{}", mod_vm.name)) {
                                            Ok(_) => {
                                                toasts.info(format!("{} disabled", mod_vm.name));
                                            }
                                            Err(_) => {
                                                toasts.error(format!(
                                                    "failed to remove {}",
                                                    mod_vm.name
                                                ));
                                            }
                                        }
                                    }
                                }
                                ui.label(mod_vm.name.to_owned());
                            });
                        });

                    if response.is_drag_finished() {
                        response.update_vec(mods);
                        // update ini
                        // add_command_to_ini(
                        //     toasts,
                        //     mods.iter()
                        //         .filter(|p| p.enabled)
                        //         .map(|p| p.name.to_owned())
                        //         .collect::<Vec<_>>()
                        //         .as_slice(),
                        // );
                    }
                });

                let mut start_command = get_command_line(
                    mods.iter()
                        .filter(|p| p.enabled)
                        .map(|p| p.name.to_owned())
                        .collect::<Vec<_>>()
                        .as_slice(),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Start commands: ");
                    ui.add_enabled_ui(false, |ui| {
                        ui.text_edit_multiline(&mut start_command);
                    });
                });
            }
        });

        // notifications
        toasts.show(ctx);
    }
}

/// Gets all txt file mods in the Data dir.
fn refresh_mods() -> Vec<ModViewModel> {
    let mut mod_map: Vec<ModViewModel> = vec![];
    for entry in read_dir("./Data").unwrap().flatten() {
        let path = entry.path();
        if path.exists() && path.is_file() {
            if let Some(name) = path.file_name() {
                if let Some(ext) = path.extension() {
                    if ext == "txt" {
                        // if the file exists in base dir then the mod is enabled
                        mod_map.push(ModViewModel {
                            path: path.to_owned(),
                            name: name.to_str().unwrap().into(),
                            enabled: PathBuf::from("./").join(name).exists(),
                        });
                    }
                }
            }
        }
    }

    // println!("before");
    // for m in mod_map.iter() {
    //     println!("{}", m.name);
    // }

    if let Some(order) = get_bat_order() {
        let mut ordered: Vec<ModViewModel> = vec![];
        for o in order {
            if let Some(found) = mod_map.iter().find(|p| p.name == o) {
                ordered.push(found.clone());
            }
        }
        let mut cnt = 0;
        for m in mod_map {
            if !ordered.iter().any(|p| p.name == m.name) {
                ordered.insert(cnt, m);
                cnt += 1;
            }
        }

        // println!("after");
        // for m in ordered.iter() {
        //     println!("{}", m.name);
        // }

        ordered
    } else {
        mod_map
    }
}

// check if a gmst mod
// if let Ok(lines) = read_lines(&path) {
//     // Consumes the iterator, returns an (Optional) String
//     for line in lines.flatten() {
//         if line.starts_with("setgs ") {
//             mod_map.push(path);
//             break;
//         }
//     }
// }

// // The output is wrapped in a Result to allow matching on errors
// // Returns an Iterator to the Reader of the lines of the file.
// fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
// where
//     P: AsRef<Path>,
// {
//     let file = File::open(filename)?;
//     Ok(io::BufReader::new(file).lines())
// }

fn get_bat_order() -> Option<Vec<String>> {
    // checks
    let Some(user_dirs) = UserDirs::new() else { return None; };
    let Some(documents) = user_dirs.document_dir() else { return None; };
    let sf_mygames_path = PathBuf::from(documents).join("My Games").join("Starfield");
    if !sf_mygames_path.exists() {
        return None;
    }
    let ini_path = sf_mygames_path.join("StarfieldCustom.ini");
    if !ini_path.exists() {
        return None;
    }

    let mut mods: Vec<String> = vec![];
    if let Ok(file) = File::open(&ini_path) {
        for line in io::BufReader::new(file).lines().flatten() {
            if let Some(args) = line.strip_prefix("sStartingConsoleCommand=") {
                let args_split: Vec<_> = args.split(';').collect();
                for arg in args_split {
                    let trimmed = arg.trim();
                    if let Some(stripped) = trimmed.strip_prefix("bat ") {
                        mods.push(stripped.to_owned());
                    }
                }
            }
        }
    }

    Some(mods)
}

/// Saves currently edited GMSTs to a file
fn save_to_file(gmst_vms: &[GmstViewModel], bat_name: &str) {
    // save to file
    if let Ok(mut file) = File::create(format!("./Data/{}.txt", bat_name)) {
        // get all edited
        for vm in gmst_vms.iter().filter(|p| p.is_edited) {
            // write to file
            let valuestring = match vm.gmst.value {
                EGmstValue::Bool(b) => b.to_string(),
                EGmstValue::Float(f) => f.to_string(),
                EGmstValue::Int(i) => i.to_string(),
                EGmstValue::UInt(u) => u.to_string(),
            };
            let line = format!("setgs {} {}", vm.gmst.name, valuestring);
            let _res = writeln!(file, "{}", line);
        }
    }
}

/// Saves all edited gmsts to a text file
/// and registers that text file in the ini
fn add_command_to_ini(toasts: &mut Toasts, commands: &[String]) {
    // checks
    let Some(user_dirs) = UserDirs::new() else { return };
    let Some(documents) = user_dirs.document_dir() else { return };
    let sf_mygames_path = PathBuf::from(documents).join("My Games").join("Starfield");
    if !sf_mygames_path.exists() {
        return;
    }
    let ini_path = sf_mygames_path.join("StarfieldCustom.ini");
    if !ini_path.exists() {
        return;
    }

    let mut needs_start_command = true;
    let mut needs_general_section = true;
    let mut ini_lines: Vec<String> = vec![];

    let start_command = get_command_line(commands);

    if let Ok(file) = File::open(&ini_path) {
        for line in io::BufReader::new(file).lines().flatten() {
            if line.starts_with("[General]") {
                // checks
                needs_general_section = false;
            }

            // modify this line
            if let Some(_args) = line.strip_prefix("sStartingConsoleCommand=") {
                needs_start_command = false;

                // collect current commands

                // let args_split: Vec<_> = args.split(';').collect();
                // for arg in args_split {
                //     let trimmed = arg.trim();
                //     if trimmed != format!("bat {}", bat_name) {
                //         collected_line += trimmed;
                //         collected_line += ";";
                //     }
                // }

                ini_lines.push(start_command.to_owned());
            } else {
                // everything else gets saved
                ini_lines.push(line);
            }
        }
    }

    if needs_general_section {
        if needs_start_command {
            ini_lines.push("".into());
            ini_lines.push("[General]".into());
            ini_lines.push(start_command.to_owned());
        } else {
            // malformed ini
            toasts.error("Ini is malformed");
        }
    }

    // write ini
    match File::create(&ini_path) {
        Ok(mut file) => {
            for line in ini_lines {
                if !needs_general_section && needs_start_command && line == *"[General]" {
                    let _res = writeln!(file, "{}", line);
                    let _res = writeln!(file, "{}", start_command);
                    continue;
                }

                let _res = writeln!(file, "{}", line);
            }
            toasts.success("Saved GMST commands");
        }
        Err(err) => {
            toasts.error(format!("{}: {}", ini_path.display(), err));
        }
    }
}

fn get_command_line(commands: &[String]) -> String {
    let mut collected_line = "".to_owned();
    for c in commands {
        collected_line += format!("bat {};", c).as_str();
    }
    let start_command = format!("sStartingConsoleCommand={}", collected_line);
    start_command
}

fn theme_switch(ui: &mut egui::Ui, theme: &mut ETheme) {
    egui::ComboBox::from_label("Theme")
        .selected_text(format!("{:?}", theme))
        .show_ui(ui, |ui| {
            ui.style_mut().wrap = Some(false);
            ui.set_min_width(60.0);
            ui.selectable_value(theme, ETheme::Latte, "LATTE");
            ui.selectable_value(theme, ETheme::Frappe, "FRAPPE");
            ui.selectable_value(theme, ETheme::Macchiato, "MACCHIATO");
            ui.selectable_value(theme, ETheme::Mocha, "MOCHA");
        });
}

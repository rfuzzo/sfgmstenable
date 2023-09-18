use egui_notify::Toasts;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::Display;
use std::io::Write;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
#[cfg(not(target_arch = "wasm32"))]
use egui::Color32;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::read_dir;

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum EGmstValue {
    Bool(bool),
    Float(f32),
    Int(i32),
    UInt(u32),
}

impl Display for EGmstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EGmstValue::Bool(b) => match b {
                true => "True".to_string(),
                false => "False".to_string(),
            },
            EGmstValue::Float(f) => f.to_string(),
            EGmstValue::Int(i) => i.to_string(),
            EGmstValue::UInt(u) => u.to_string(),
        };
        write!(f, "{}", s)
    }
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
    pub overlay_enabled: bool,
    pub gmsts: Vec<String>,
    pub txt: Option<String>,
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

/// App scale
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EScale {
    Native,
    Small,
    Medium,
    Large,
}
impl From<EScale> for f32 {
    fn from(val: EScale) -> Self {
        match val {
            EScale::Native => 1.2,
            EScale::Small => 2.0,
            EScale::Medium => 3.0,
            EScale::Large => 4.0,
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    pub theme: ETheme,
    pub scale: EScale,

    // ui
    #[serde(skip)]
    pub toasts: Toasts,

    #[serde(skip)]
    pub mods: Option<Vec<ModViewModel>>,
    #[serde(skip)]
    pub default_gmsts: HashMap<String, EGmstValue>,
    #[serde(skip)]
    pub gmst_vms: Vec<GmstViewModel>,
    // #[serde(skip)]
    // pub mod_gmst_vms: Option<HashMap<String, EGmstValue>>,

    // runtime
    #[serde(skip)]
    pub search_filter: String,
    #[serde(skip)]
    pub display_edited: bool,
    #[serde(skip)]
    pub selected_mod: Option<ModViewModel>,
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
            //mod_gmst_vms: None,
            scale: EScale::Small,
            selected_mod: None,
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

    let bytes1 = include_bytes!("Starfield_Game_Settings.csv");
    add_from_bytes(bytes1, &mut map);

    let bytes2 = include_bytes!("ghidra_gmsts.csv");
    add_from_bytes(bytes2, &mut map);

    map
}

fn add_from_bytes(bytes: &[u8], map: &mut HashMap<String, EGmstValue>) {
    let reader = io::BufReader::new(bytes);
    reader.lines().for_each(|line| {
        if let Ok(str) = line {
            // parse first char
            let split: Vec<_> = str.split(',').collect();
            if split.len() == 2 {
                let name = split[0].trim();
                let value = split[1].trim();
                if let Some(parsed) = parse_gmst(name, value) {
                    map.entry(name.to_owned()).or_insert(parsed);
                }
            }
        }
    });
}

fn parse_gmst(name: &str, value: &str) -> Option<EGmstValue> {
    let first_char: char = name.chars().next().unwrap();

    match first_char {
        'b' => {
            // parse bool
            match value.to_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
            .map(EGmstValue::Bool)
        }
        'f' => {
            // parse float
            if let Ok(parsed) = value.parse::<f32>() {
                Some(EGmstValue::Float(parsed))
            } else {
                None
            }
        }
        'i' => {
            // parse float
            if let Ok(parsed) = value.parse::<i32>() {
                Some(EGmstValue::Int(parsed))
            } else {
                None
            }
        }
        'u' => {
            // parse float
            if let Ok(parsed) = value.parse::<u32>() {
                Some(EGmstValue::UInt(parsed))
            } else {
                None
            }
        }
        _ => None,
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
static ROW_WIDTH: f32 = 50_f32;
static BAT_NAME: &str = "my_gmsts";
//static BAT_NAME_MERGED: &str = "merged_gmsts";

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// wasm
    #[cfg(target_arch = "wasm32")]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            theme,
            mods: mods_option,
            toasts,
            default_gmsts,
            gmst_vms,
            search_filter,
            display_edited,
            scale,
            selected_mod,
        } = self;

        catppuccin_egui::set_theme(ctx, get_theme(theme));

        egui::CentralPanel::default().show(ctx, |ui| {
            // Headers
            ui.heading("GMSTs");
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

                            // get values
                            if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                vm.is_edited = !default_value.eq(&vm.gmst.value);
                            }

                            if *display_edited && !vm.is_edited {
                                continue;
                            }

                            if vm.is_edited {
                                ui.visuals_mut().override_text_color = Some(egui::Color32::GREEN);
                            } else {
                                ui.visuals_mut().override_text_color = None;
                            }

                            // edited checkbox
                            ui.add_enabled_ui(false, |ui| {
                                ui.checkbox(&mut vm.is_edited, "Edited");
                            });

                            // mod name
                            let mut mod_name = vm.gmst.name.to_owned();
                            if vm.is_edited {
                                if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                    mod_name = format!("{} ({})", mod_name, default_value);
                                }
                            }
                            ui.label(mod_name);

                            // mod value
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

                            // Reset
                            if vm.is_edited && ui.button("Reset").clicked() {
                                if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                    vm.gmst.value = *default_value;
                                }
                            }

                            ui.end_row();
                        }
                    });
            });
        });
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    #[cfg(not(target_arch = "wasm32"))]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            theme,
            mods: mods_option,
            toasts,
            default_gmsts,
            gmst_vms,
            search_filter,
            display_edited,
            scale,
            selected_mod,
        } = self;

        ctx.set_pixels_per_point(f32::from(*scale));
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
                    theme_switch(ui, theme);
                    // scale
                    egui::ComboBox::from_label("Scale: ")
                        .selected_text(format!("{:?}", scale))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(scale, EScale::Native, "Native");
                            ui.selectable_value(scale, EScale::Small, "Small");
                            ui.selectable_value(scale, EScale::Medium, "Medium");
                            ui.selectable_value(scale, EScale::Large, "Large");
                        });
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        if let Ok(cwd) = env::current_dir() {
            if !cwd.join("Starfield.exe").exists() {
                // then we are in the wrong dir
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(format!("Starfield GMST editor v{}", VERSION));
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
                    let save_path = PathBuf::from("").join(format!("{}.txt", BAT_NAME));

                    // save file
                    if ui
                        .button(
                            egui::RichText::new("🖹 Create command file")
                                .size(14.0)
                                .color(Color32::GREEN),
                        )
                        .clicked()
                    {
                        let map = gmst_vms
                            .iter()
                            .filter(|p| p.is_edited)
                            .map(|p| (p.gmst.name.to_owned(), p.gmst.value))
                            .collect::<HashMap<String, EGmstValue>>();
                        save_to_file(&map, &save_path);

                        // refresh UI
                        *mods_option = Some(refresh_mods());
                        if let Some(selected_mod) = selected_mod {
                            if selected_mod.path == save_path {
                                if let Ok(txt) =
                                    std::fs::read_to_string(format!("{}.txt", BAT_NAME))
                                {
                                    selected_mod.txt = Some(txt);
                                }
                            }
                        }

                        toasts.success(format!("Created file: {}", save_path.display()));
                    }

                    // append to file
                    ui.add_enabled_ui(save_path.exists(), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("➕ Append to command file")
                                    .size(14.0)
                                    .color(Color32::GREEN),
                            )
                            .clicked()
                        {
                            let mut new_gmsts = parse_file(&save_path);
                            // add currently edited gmsts
                            for g in gmst_vms.iter().filter(|p| p.is_edited) {
                                new_gmsts.insert(g.gmst.name.to_owned(), g.gmst.value);
                            }

                            save_to_file(&new_gmsts, &save_path);

                            if let Some(selected_mod) = selected_mod {
                                if selected_mod.path == save_path {
                                    if let Ok(txt) =
                                        std::fs::read_to_string(format!("{}.txt", BAT_NAME))
                                    {
                                        selected_mod.txt = Some(txt);
                                    }
                                }
                            }

                            toasts.success(format!("Appended to file: {}", save_path.display()));
                        }
                    });
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

                            // get values
                            if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                vm.is_edited = !default_value.eq(&vm.gmst.value);
                            }

                            if *display_edited && !vm.is_edited {
                                continue;
                            }

                            if vm.is_edited {
                                ui.visuals_mut().override_text_color = Some(egui::Color32::GREEN);
                            } else {
                                ui.visuals_mut().override_text_color = None;
                            }

                            // edited checkbox
                            ui.add_enabled_ui(false, |ui| {
                                ui.checkbox(&mut vm.is_edited, "Edited");
                            });

                            // mod name
                            let mut mod_name = vm.gmst.name.to_owned();
                            if vm.is_edited {
                                if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                    mod_name = format!("{} ({})", mod_name, default_value);
                                }
                            }
                            ui.label(mod_name);

                            // mod value
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

                            // Reset
                            if vm.is_edited && ui.button("Reset").clicked() {
                                if let Some(default_value) = default_gmsts.get(&vm.gmst.name) {
                                    vm.gmst.value = *default_value;
                                }
                            }

                            ui.end_row();
                        }
                    });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Starfield GMST editor v{}", VERSION));
            ui.hyperlink("https://github.com/rfuzzo/sfgmstenable");
            ui.separator();

            // mods grid
            ui.heading("Active mods");
            ui.label("Change load order by reordering.");
            if let Some(mods) = mods_option {
                ui.horizontal(|ui| {
                    if ui.button("↻ Refresh").clicked() {
                        *mods = refresh_mods();
                    }
                    if ui.button("🗁 Open folder").clicked() {
                        let path = PathBuf::from("");
                        let _r = open::that(path);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response =
                        egui_dnd::dnd(ui, "dnd").show_vec(mods, |ui, mod_vm, handle, _dragging| {
                            ui.horizontal(|ui| {
                                handle.ui(ui, |ui| {
                                    ui.label("::");
                                });

                                // enabled checkbox
                                if ui.checkbox(&mut mod_vm.enabled, "").clicked() {
                                    if mod_vm.enabled {
                                        // copy file
                                        toasts.success(format!("{} enabled", mod_vm.name));
                                    } else {
                                        // delete file
                                        toasts.info(format!("{} disabled", mod_vm.name));
                                    }
                                }

                                // mod name
                                ui.label(mod_vm.name.to_owned());

                                // show text
                                if ui.button("🖹").clicked() {
                                    if let Ok(txt) = std::fs::read_to_string(&mod_vm.path) {
                                        mod_vm.txt = Some(txt);
                                        *selected_mod = Some(mod_vm.to_owned());
                                    }
                                }

                                // toggle show mod values
                                if ui
                                    .toggle_value(&mut mod_vm.overlay_enabled, "Toggle show")
                                    .clicked()
                                {
                                    if mod_vm.overlay_enabled {
                                        let map = parse_file(&mod_vm.path);
                                        mod_vm.gmsts =
                                            map.iter().map(|f| f.0.to_owned()).collect::<Vec<_>>();
                                        for gmst in map {
                                            // change values
                                            if let Some(val) =
                                                gmst_vms.iter_mut().find(|p| p.gmst.name == gmst.0)
                                            {
                                                val.gmst.value = gmst.1;
                                            }
                                        }
                                    } else {
                                        // revert
                                        for g in mod_vm.gmsts.iter_mut() {
                                            if let Some(val) =
                                                gmst_vms.iter_mut().find(|p| p.gmst.name == *g)
                                            {
                                                if let Some(default_value) = default_gmsts.get(g) {
                                                    val.gmst.value = *default_value;
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        });

                    if response.is_drag_finished() {
                        response.update_vec(mods);
                    }
                });

                // start commandline
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
                    //ui.add_enabled_ui(false, |ui| {
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut start_command),
                    );
                    //});
                });
                ui.horizontal(|ui| {
                    if ui.button("💾 Save to ini").clicked() {
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

                // file text
                if let Some(selected_mod) = selected_mod {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(selected_mod.name.to_owned())
                            .strong()
                            .size(14_f32),
                    );
                    if let Some(mod_text) = selected_mod.txt.to_owned() {
                        ui.push_id("text_scroll", |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                //ui.add_enabled_ui(false, |ui| {
                                ui.add_sized(
                                    ui.available_size(),
                                    egui::TextEdit::multiline(&mut mod_text.as_str()),
                                );
                                //});
                            });
                        });
                    }
                }
            }
        });

        // notifications
        toasts.show(ctx);
    }
}

/// Parse a file for gmsts
#[cfg(not(target_arch = "wasm32"))]
fn parse_file(path: &PathBuf) -> HashMap<String, EGmstValue> {
    let mut map: HashMap<String, EGmstValue> = HashMap::default();
    if let Ok(lines) = read_lines(path) {
        for line in lines.flatten() {
            let lline = line.to_lowercase();
            if lline.starts_with("setgs ") {
                let splits = &line["setgs ".len()..].split(' ').collect::<Vec<_>>();
                if splits.len() == 2 {
                    let name = splits[0].trim_matches('"');
                    if let Some(parsed_value) = parse_gmst(name, splits[1]) {
                        map.insert(name.to_owned(), parsed_value);
                    }
                }
            }
        }
    }
    map
}

/// Gets all txt file mods in the base dir.
#[cfg(not(target_arch = "wasm32"))]
fn refresh_mods() -> Vec<ModViewModel> {
    let mut mod_map: Vec<ModViewModel> = vec![];
    let path = PathBuf::from("");

    for entry in read_dir(path).unwrap().flatten() {
        let path = entry.path();
        if path.exists() && path.is_file() {
            if let Some(name) = path.file_name() {
                if let Some(ext) = path.extension() {
                    if ext == "txt" {
                        // if the file exists in base dir then the mod is enabled
                        mod_map.push(ModViewModel {
                            path: path.to_owned(),
                            name: name.to_str().unwrap().into(),
                            enabled: false,
                            overlay_enabled: false,
                            gmsts: vec![],
                            txt: None,
                        });
                    }
                }
            }
        }
    }

    // sort by load order
    if let Some(order) = get_bat_order() {
        let mut ordered: Vec<ModViewModel> = vec![];
        for o in order {
            if let Some(found) = mod_map.iter_mut().find(|p| p.name == format!("{}.txt", o)) {
                found.enabled = true;
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

        ordered
    } else {
        mod_map
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
#[cfg(not(target_arch = "wasm32"))]
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<std::path::Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
fn save_to_file(gmst_vms: &HashMap<String, EGmstValue>, path: &PathBuf) {
    // save to file
    if let Ok(mut file) = File::create(path) {
        // get all edited
        let mut gmsts = gmst_vms.iter().collect::<Vec<_>>();
        gmsts.sort_by(|a, b| a.0.cmp(b.0));

        for vm in gmsts {
            // write to file
            let valuestring = match vm.1 {
                EGmstValue::Bool(b) => b.to_string(),
                EGmstValue::Float(f) => f.to_string(),
                EGmstValue::Int(i) => i.to_string(),
                EGmstValue::UInt(u) => u.to_string(),
            };
            let line = match vm.0.contains(':') {
                true => format!("setgs \"{}\" {}", vm.0, valuestring),
                false => format!("setgs {} {}", vm.0, valuestring),
            };
            let _res = writeln!(file, "{}", line);
        }
    }
}

/// Saves all edited gmsts to a text file
/// and registers that text file in the ini
#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn get_command_line(commands: &[String]) -> String {
    let mut collected_line = "".to_owned();
    for c in commands {
        let mut name = c.to_owned();
        if c.ends_with(".txt") {
            name = name[..c.len() - 4].to_owned();
        }
        collected_line += format!("bat {};", name).as_str();
    }
    let start_command = format!("sStartingConsoleCommand={}", collected_line);
    start_command
}

#[cfg(not(target_arch = "wasm32"))]
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

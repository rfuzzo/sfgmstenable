use egui_notify::Toasts;
use std::collections::HashMap;
use std::env;

use crate::{parse_gmsts, EGmstValue, EScale, ETheme, Gmst, GmstViewModel, ModViewModel};

#[cfg(not(target_arch = "wasm32"))]
use egui::Color32;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    pub theme: ETheme,
    pub scale: EScale,
    pub use_ccr: bool,

    // ui
    #[serde(skip)]
    pub toasts: Toasts,

    #[serde(skip)]
    pub mods: Option<Vec<ModViewModel>>,
    #[serde(skip)]
    pub ccr_mods: Option<Vec<ModViewModel>>,

    #[serde(skip)]
    pub default_gmsts: HashMap<String, EGmstValue>,
    #[serde(skip)]
    pub gmst_vms: Vec<GmstViewModel>,

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
            ccr_mods: None,
            toasts: Toasts::default(),
            default_gmsts: parse_gmsts(),
            gmst_vms: vec![],
            search_filter: "".to_owned(),
            display_edited: false,
            scale: EScale::Small,
            selected_mod: None,
            use_ccr: false,
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

const VERSION: &str = env!("CARGO_PKG_VERSION");
static BAT_NAME: &str = "my_gmsts";

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
            ccr_mods: ccr_mods_option,
            toasts,
            default_gmsts,
            gmst_vms,
            search_filter,
            display_edited,
            scale,
            selected_mod,
            use_ccr,
        } = self;

        //catppuccin_egui::set_theme(ctx, get_theme(theme));

        egui::CentralPanel::default().show(ctx, |ui| {
            show_gmst_list_only(ui, search_filter, display_edited, gmst_vms, default_gmsts);
        });
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    #[cfg(not(target_arch = "wasm32"))]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use crate::{
            add_command_to_ini, get_command_line, get_mod_file_path, get_mods_folder, parse_file,
            refresh_mods, save_to_file,
        };

        let Self {
            theme,
            mods: mods_option,
            ccr_mods: ccr_mods_option,
            toasts,
            default_gmsts,
            gmst_vms,
            search_filter,
            display_edited,
            scale,
            selected_mod,
            use_ccr,
        } = self;

        ctx.set_pixels_per_point(f32::from(*scale));
        //catppuccin_egui::set_theme(ctx, get_theme(theme));

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
                    ui.separator();

                    show_gmst_list_only(ui, search_filter, display_edited, gmst_vms, default_gmsts );
                });
                return;
            }
        }

        // fill ist of mods
        // TODO refactor this
        if mods_option.is_none() {
            *mods_option = Some(refresh_mods(false));
        }
        if ccr_mods_option.is_none() {
            *ccr_mods_option = Some(refresh_mods(true));
        }

        egui::SidePanel::left("left_panel_id").show(ctx, |ui| {
            // Headers
            ui.heading("GMSTs");
            // save buttons
            ui.add_enabled_ui(gmst_vms.iter().any(|p| p.is_edited), |ui| {
                ui.horizontal(|ui| {
                    let save_path = get_mod_file_path(*use_ccr, BAT_NAME);

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
                        save_to_file(toasts, &map, &save_path, *use_ccr);

                        // refresh UI
                        if *use_ccr {
                            *ccr_mods_option = Some(refresh_mods(true));
                        } else {
                            *mods_option = Some(refresh_mods(false));
                        }

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
                            let mut new_gmsts = parse_file(&save_path, *use_ccr);
                            // add currently edited gmsts
                            for g in gmst_vms.iter().filter(|p| p.is_edited) {
                                new_gmsts.insert(g.gmst.name.to_owned(), g.gmst.value);
                            }

                            save_to_file(toasts, &new_gmsts, &save_path, *use_ccr);

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

                    // use CCR
                    ui.checkbox(use_ccr, "Use CCR");
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
            egui::ScrollArea::horizontal().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("main_grid_id")
                        .num_columns(3)
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
                                    ui.visuals_mut().override_text_color =
                                        Some(egui::Color32::GREEN);
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
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Starfield GMST editor v{}", VERSION));
            ui.hyperlink("https://github.com/rfuzzo/sfgmstenable");
            ui.separator();

            // mods table
            ui.heading("Active mods");
            ui.label("Change load order by reordering.");
            if let Some(mods) = mods_option {
                ui.horizontal(|ui| {
                    if ui.button("↻ Refresh").clicked() {
                        *mods = refresh_mods(false);
                    }
                    if ui.button("🗁 Open folder").clicked() {
                        if let Err(err) = open::that(get_mods_folder(false)) {
                            toasts.error(format!("Could not open folder: {}", err));
                        }
                    }
                    if ui.button("💾 Save to ini").clicked() {
                        if let Err(err) = add_command_to_ini(
                            mods.iter()
                                .filter(|p| p.enabled)
                                .map(|p| p.name.to_owned())
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ) {
                            toasts.error(format!("Failed to save to ini: {}", err));
                        }
                    }
                });
                ui.separator();
                ui.push_id("main_grid_bat_scroll_id", |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let response = egui_dnd::dnd(ui, "dnd").show_vec(
                            mods,
                            |ui, mod_vm, handle, _dragging| {
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
                                        toggle_mod_values(mod_vm, gmst_vms, default_gmsts, false);
                                    }
                                });
                            },
                        );

                        if response.is_drag_finished() {
                            response.update_vec(mods);
                        }
                    });
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
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut start_command),
                    );
                });
            }

            // CCR table
            ui.separator();
            ui.heading("CCR mods");
            if let Some(ccr_mods) = ccr_mods_option {
                ui.horizontal(|ui| {
                    if ui.button("↻ Refresh").clicked() {
                        *ccr_mods = refresh_mods(true);
                    }
                    if ui.button("🗁 Open folder").clicked() {
                        if let Err(err) = open::that(get_mods_folder(true)) {
                            toasts.error(format!("Could not open folder: {}", err));
                        }
                    }
                });
                ui.separator();
                ui.push_id("main_grid_ccr_scroll_id", |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("main_grid_ccr_id")
                            .num_columns(3)
                            .show(ui, |ui| {
                                for mod_vm in ccr_mods {
                                    ui.horizontal(|ui| {
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
                                            .toggle_value(
                                                &mut mod_vm.overlay_enabled,
                                                "Toggle show",
                                            )
                                            .clicked()
                                        {
                                            toggle_mod_values(
                                                mod_vm,
                                                gmst_vms,
                                                default_gmsts,
                                                true,
                                            );
                                        }
                                    });
                                    ui.end_row();
                                }
                            });
                    });
                });
            }

            // file text
            //ui.separator();
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
                            ui.add_sized(
                                ui.available_size(),
                                egui::TextEdit::multiline(&mut mod_text.as_str()),
                            );
                        });
                    });
                }
            }
        });

        // notifications
        toasts.show(ctx);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_mod_values(
    mod_vm: &mut ModViewModel,
    gmst_vms: &mut [GmstViewModel],
    default_gmsts: &mut HashMap<String, EGmstValue>,
    is_ccr: bool,
) {
    use crate::parse_file;

    if mod_vm.overlay_enabled {
        let map = parse_file(&mod_vm.path, is_ccr);
        mod_vm.gmsts = map.iter().map(|f| f.0.to_owned()).collect::<Vec<_>>();
        for gmst in map {
            // change values
            if let Some(val) = gmst_vms.iter_mut().find(|p| p.gmst.name == gmst.0) {
                val.gmst.value = gmst.1;
            }
        }
    } else {
        // revert
        for g in mod_vm.gmsts.iter_mut() {
            if let Some(val) = gmst_vms.iter_mut().find(|p| p.gmst.name == *g) {
                if let Some(default_value) = default_gmsts.get(g) {
                    val.gmst.value = *default_value;
                }
            }
        }
    }
}

fn show_gmst_list_only(
    ui: &mut egui::Ui,
    search_filter: &mut String,
    display_edited: &mut bool,
    gmst_vms: &mut [GmstViewModel],
    default_gmsts: &HashMap<String, EGmstValue>,
) {
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
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("main_grid_id")
                .num_columns(3)
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

#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::TemplateApp;
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
};

#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
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
pub enum EModType {
    BatMod,
    CcrMod,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone)]
pub struct ModViewModel {
    pub mod_type: EModType,
    pub path: PathBuf,
    pub name: String,
    pub enabled: bool,
    /// Show mod values in GMST view
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
// pub fn get_theme(theme: &ETheme) -> catppuccin_egui::Theme {
//     match theme {
//         ETheme::Frappe => catppuccin_egui::FRAPPE,
//         ETheme::Latte => catppuccin_egui::LATTE,
//         ETheme::Macchiato => catppuccin_egui::MACCHIATO,
//         ETheme::Mocha => catppuccin_egui::MOCHA,
//     }
// }

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

fn parse_gmsts() -> HashMap<String, EGmstValue> {
    let mut map: HashMap<String, EGmstValue> = HashMap::default();

    let bytes1 = include_bytes!("Starfield_Game_Settings.csv");
    add_from_bytes(bytes1, &mut map);

    let bytes2 = include_bytes!("ghidra_gmsts.csv");
    add_from_bytes(bytes2, &mut map);

    map
}

/// Parse a file for gmsts
#[cfg(not(target_arch = "wasm32"))]
fn parse_file(path: &PathBuf, is_ccr: bool) -> HashMap<String, EGmstValue> {
    let mut commands: Vec<String> = vec![];

    if is_ccr {
        // deserialize toml
        if let Ok(file_contents) = std::fs::read_to_string(path) {
            let res: CcrModel = toml::from_str(file_contents.as_str()).unwrap();
            for event in res.event {
                for command in event.commands {
                    commands.push(command);
                }
            }
        }
    } else if let Ok(lines) = read_lines(path) {
        for line in lines.flatten() {
            commands.push(line);
        }
    }

    let mut map: HashMap<String, EGmstValue> = HashMap::default();
    for c in commands {
        let lline = c.to_lowercase();
        if lline.starts_with("setgs ") {
            let splits = &c["setgs ".len()..].split(' ').collect::<Vec<_>>();
            if splits.len() == 2 {
                let name = splits[0].trim_matches('"');
                if let Some(parsed_value) = parse_gmst(name, splits[1]) {
                    map.insert(name.to_owned(), parsed_value);
                }
            }
        }
    }

    map
}

#[derive(Default, Serialize, Deserialize)]
pub enum CCrEEventType {
    #[default]
    DataLoaded,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CcrEvent {
    pub event_type: CCrEEventType,
    pub commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CcrModel {
    pub event: Vec<CcrEvent>,
}

#[cfg(not(target_arch = "wasm32"))]
fn get_mods_folder(is_ccr: bool) -> PathBuf {
    if is_ccr {
        PathBuf::from("./")
            .join("Data")
            .join("SFSE")
            .join("Plugins")
            .join("ConsoleCommandRunner")
    } else {
        PathBuf::from("./")
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_mod_file_path(is_ccr: bool, file_name: &str) -> PathBuf {
    if is_ccr {
        get_mods_folder(is_ccr).join(format!("{}.toml", file_name))
    } else {
        get_mods_folder(is_ccr).join(format!("{}.txt", file_name))
    }
}

/// Gets all txt file mods in the base dir.
#[cfg(not(target_arch = "wasm32"))]
fn refresh_mods(is_ccr: bool) -> Vec<ModViewModel> {
    if is_ccr {
        refresh_ccr_mods()
    } else {
        refresh_bat_mods()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn refresh_ccr_mods() -> Vec<ModViewModel> {
    let mut mod_map: Vec<ModViewModel> = vec![];
    let path = get_mods_folder(true);
    if !path.exists() {
        return mod_map;
    }

    for entry in read_dir(path).unwrap().flatten() {
        let path = entry.path();
        if path.exists() && path.is_file() {
            if let Some(name) = path.file_name() {
                if let Some(ext) = path.extension() {
                    if ext == "toml" {
                        // if the file exists in base dir then the mod is enabled
                        mod_map.push(ModViewModel {
                            mod_type: crate::EModType::CcrMod,
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

    mod_map.sort_by_key(|k| k.name.to_owned());
    mod_map
}

#[cfg(not(target_arch = "wasm32"))]
fn refresh_bat_mods() -> Vec<ModViewModel> {
    let mut mod_map: Vec<ModViewModel> = vec![];
    for entry in read_dir(get_mods_folder(false)).unwrap().flatten() {
        let path = entry.path();
        if path.exists() && path.is_file() {
            if let Some(name) = path.file_name() {
                if let Some(ext) = path.extension() {
                    if ext == "txt" {
                        // if the file exists in base dir then the mod is enabled
                        mod_map.push(ModViewModel {
                            mod_type: crate::EModType::BatMod,
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

#[cfg(not(target_arch = "wasm32"))]
fn get_bat_order() -> Option<Vec<String>> {
    // checks
    let Some(user_dirs) = UserDirs::new() else {
        return None;
    };
    let Some(documents) = user_dirs.document_dir() else {
        return None;
    };
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
fn save_to_file(
    toasts: &mut egui_notify::Toasts,
    gmst_vms: &HashMap<String, EGmstValue>,
    path: &PathBuf,
    use_ccr: bool,
) {
    // save to file

    if let Ok(mut file) = File::create(path) {
        // get all edited
        let mut gmsts = gmst_vms.iter().collect::<Vec<_>>();
        gmsts.sort_by(|a, b| a.0.cmp(b.0));

        let mut commands: Vec<String> = vec![];
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
            commands.push(line);
        }

        if use_ccr {
            let event: CcrEvent = CcrEvent {
                commands,
                ..Default::default()
            };
            let events: Vec<crate::CcrEvent> = vec![event];
            let model = CcrModel { event: events };
            let toml = toml::to_string_pretty(&model).unwrap();
            if let Err(err) = write!(file, "{}", toml) {
                toasts.error(format!("Failed to write toml: {}", err));
            }
        } else {
            for line in commands {
                if let Err(err) = writeln!(file, "{}", line) {
                    toasts.error(format!("Failed to write line: {}", err));
                }
            }
        }
    }
}

/// Saves all edited gmsts to a text file
/// and registers that text file in the ini
#[cfg(not(target_arch = "wasm32"))]
fn add_command_to_ini(commands: &[String]) -> io::Result<()> {
    // checks
    use std::io::{Error, ErrorKind};

    let user_dirs = UserDirs::new().ok_or(Error::new(ErrorKind::NotFound, "UserDirs not found"))?;
    let documents = user_dirs
        .document_dir()
        .ok_or(Error::new(ErrorKind::NotFound, "document_dir not found"))?;
    let sf_mygames_path = PathBuf::from(documents).join("My Games").join("Starfield");
    if !sf_mygames_path.exists() {
        return Err(Error::new(ErrorKind::NotFound, "My Games not found"));
    }
    let ini_path = sf_mygames_path.join("StarfieldCustom.ini");
    if !ini_path.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "StarfieldCustom.ini not found",
        ));
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
            //toasts.error("Ini is malformed");
        }
    }

    // write ini
    match File::create(&ini_path) {
        Ok(mut file) => {
            for line in ini_lines {
                if !needs_general_section && needs_start_command && line == *"[General]" {
                    writeln!(file, "{}", line)?;
                    writeln!(file, "{}", start_command)?;
                    continue;
                }

                writeln!(file, "{}", line)?;
            }

            Ok(())
        }
        Err(err) => Err(err),
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

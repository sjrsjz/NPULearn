use std::collections::HashMap;
use std::fs::File;
use serde::{de::value::Error, Deserialize, Serialize};

static FILE_PATH: &str = "./resources/chatHistory.json";

#[derive(Clone, Serialize, Deserialize)]
#[derive(Debug)]
pub(crate) struct ChatHistory {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) time: String,
    pub(crate) content: String,
}

#[tauri::command]
pub fn load_history() -> HashMap<u32, ChatHistory> {
    println!("Current work path: {}", std::env::current_dir().unwrap().display());

    // Try to open the file, if it doesn't exist, create a new one
    let file = match File::open(FILE_PATH) {
        Ok(file) => {
            // Check if file is empty
            if file.metadata().unwrap().len() == 0 {
                return HashMap::new();
            }
            file
        }
        Err(e) => {
            // Create a new file and return empty HashMap
            File::create(FILE_PATH).unwrap();
            println!("Filed to open file: {}", e);
            return HashMap::new();
        }
    };

    // Parse JSON from file
    let contents: HashMap<u32, ChatHistory> = serde_json::from_reader(file).unwrap();
    contents
}

#[tauri::command]
pub fn save_history(history: HashMap<u32, ChatHistory>) {
    let file = File::create(FILE_PATH).unwrap();
    serde_json::to_writer(file, &history).unwrap();
}

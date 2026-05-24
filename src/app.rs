use std::path::{Path, PathBuf};

use crate::rules::RenameRule;

pub struct App {
    pub running: bool,
    pub current_dir: PathBuf,
    pub files: Vec<String>,
    pub file_cursor: usize,
    pub file_scroll: usize,
    pub rules: Vec<RenameRule>,
    pub rule_cursor: usize,
    pub active_panel: Panel,
    pub status_msg: String,
    pub error_msg: Option<String>,
    pub dialog_mode: RuleDialogMode,
    pub dialog_cursor: usize,
    pub rule_input_buffer: String,
    pub rule_input_step: RuleInputStep,
    pub find_pattern: Option<String>,
    pub numbering_start: Option<u32>,
    pub numbering_width: Option<usize>,
    pub suffix_after_extension: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Rules,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleDialogMode {
    None,
    SelectRule,
    FindReplace,
    FindReplaceRegex,
    Prefix,
    Suffix,
    RemovePattern,
    Numbering,
    Case,
    ConfirmQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleInputStep {
    Waiting,
    InputText,
    InputReplace,
    InputNumber,
    InputWidth,
    InputPlaceholder,
    SelectCase,
}

impl App {
    pub fn new() -> Self {
        let current_dir = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let files = load_files(&current_dir);

        Self {
            running: true,
            current_dir,
            files,
            file_cursor: 0,
            file_scroll: 0,
            rules: Vec::new(),
            rule_cursor: 0,
            active_panel: Panel::Files,
            status_msg: String::from("ern - EncryptedData's ReNamer | arrows:navigate  tab:panel  enter:Add Rule  d:del  ctrl+w:rename  q:quit"),
            error_msg: None,
            dialog_mode: RuleDialogMode::None,
            dialog_cursor: 0,
            rule_input_buffer: String::new(),
            rule_input_step: RuleInputStep::Waiting,
            find_pattern: None,
            numbering_start: None,
            numbering_width: None,
            suffix_after_extension: false,
        }
    }

    pub fn refresh_files(&mut self) {
        self.files = load_files(&self.current_dir);
        if self.file_cursor >= self.files.len() {
            self.file_cursor = self.files.len().saturating_sub(1);
        }
        if self.file_scroll >= self.files.len() {
            self.file_scroll = self.files.len().saturating_sub(1);
        }
    }

    pub fn rename_files(&mut self) -> Vec<(String, String)> {
        let mut results = Vec::new();
        let mut file_index: u32 = 0;

        for file in self.files.iter() {
            let old_name = file;
            let new_name = crate::rules::apply_rules(old_name, &self.rules, file_index);
            results.push((old_name.clone(), new_name.clone()));

            let old_path = self.current_dir.join(old_name);
            let new_path = self.current_dir.join(&new_name);
            match std::fs::rename(&old_path, &new_path) {
                Ok(()) => {}
                Err(e) => {
                    self.error_msg = Some(format!("Failed to rename '{}': {}", old_name, e));
                }
            }
            file_index += 1;
        }

        self.refresh_files();

        results
    }

    pub fn add_rule(&mut self, rule: RenameRule) {
        self.rules.push(rule);
        self.rule_cursor = self.rules.len();
        self.status_msg = String::from("Rule added.");
    }

    pub fn remove_rule(&mut self) {
        if self.rule_cursor < self.rules.len() {
            self.rules.remove(self.rule_cursor);
            self.status_msg = String::from("Rule removed.");
        }
    }

    pub fn clear_input(&mut self) {
        self.dialog_mode = RuleDialogMode::None;
        self.rule_input_step = RuleInputStep::Waiting;
        self.rule_input_buffer.clear();
        self.find_pattern = None;
        self.numbering_start = None;
        self.numbering_width = None;
        self.suffix_after_extension = false;
    }
}

fn load_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if !name.starts_with('.') {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    files
}

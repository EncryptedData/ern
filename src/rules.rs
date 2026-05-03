use std::fmt;

#[derive(Debug, Clone)]
pub enum RenameRule {
    FindReplace {
        find: String,
        replace: String,
    },
    FindReplaceRegex {
        pattern: String,
        replacement: String,
    },
    AddPrefix(String),
    AddSuffix(String),
    ChangeCase(CaseTransform),
    RemovePattern(String),
    Numbering {
        start: u32,
        width: usize,
        placeholder: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum CaseTransform {
    Upper,
    Lower,
    Title,
    Toggle,
}

impl fmt::Display for CaseTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaseTransform::Upper => write!(f, "UPPER"),
            CaseTransform::Lower => write!(f, "lower"),
            CaseTransform::Title => write!(f, "Title"),
            CaseTransform::Toggle => write!(f, "tOGGLE"),
        }
    }
}

impl fmt::Display for RenameRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenameRule::FindReplace { find, replace } => {
                write!(f, "find '{}' → '{}'", find, replace)
            }
            RenameRule::FindReplaceRegex {
                pattern,
                replacement,
            } => {
                write!(f, "[rx]find '{}' → '{}'", pattern, replacement)
            }
            RenameRule::AddPrefix(s) => write!(f, "prefix '{}'", s),
            RenameRule::AddSuffix(s) => write!(f, "suffix '{}'", s),
            RenameRule::ChangeCase(t) => write!(f, "case {}", t),
            RenameRule::RemovePattern(s) => write!(f, "remove '{}'", s),
            RenameRule::Numbering {
                start,
                width: _,
                placeholder,
            } => {
                write!(f, "number {}.. ({})", start, placeholder)
            }
        }
    }
}

impl RenameRule {
    pub fn apply(&self, filename: &str, index: u32) -> String {
        let (name, ext) = split_filename(filename);
        let mut new_name = name.to_string();

        match self {
            RenameRule::FindReplace { find, replace } => {
                new_name = new_name.replace(find, replace);
            }
            RenameRule::FindReplaceRegex {
                pattern,
                replacement,
            } => {
                if let Some(re) = regex::Regex::new(pattern).ok() {
                    new_name = re.replace_all(&new_name, replacement.as_str()).to_string();
                }
            }
            RenameRule::AddPrefix(s) => {
                new_name = format!("{}{}", s, new_name);
            }
            RenameRule::AddSuffix(s) => {
                new_name = format!("{}{}", new_name, s);
            }
            RenameRule::ChangeCase(t) => {
                new_name = apply_case(&new_name, t);
            }
            RenameRule::RemovePattern(s) => {
                if let Some(re) = regex::Regex::new(s).ok() {
                    new_name = re.replace_all(&new_name, "").to_string();
                } else {
                    new_name = new_name.replace(s, "");
                }
            }
            RenameRule::Numbering {
                start,
                width,
                placeholder,
            } => {
                let num_str = format!("{:0>width$}", start + index, width = *width);
                new_name = new_name.replace(placeholder, &num_str);
            }
        }

        if ext.is_empty() {
            new_name
        } else {
            format!("{}.{}", new_name, ext)
        }
    }

    #[allow(dead_code)]
    pub fn apply_to_stem(&self, stem: &str, index: u32) -> String {
        let mut new_name = stem.to_string();

        match self {
            RenameRule::FindReplace { find, replace } => {
                new_name = new_name.replace(find, replace);
            }
            RenameRule::FindReplaceRegex {
                pattern,
                replacement,
            } => {
                if let Some(re) = regex::Regex::new(pattern).ok() {
                    new_name = re.replace_all(&new_name, replacement.as_str()).to_string();
                }
            }
            RenameRule::AddPrefix(s) => {
                new_name = format!("{}{}", s, new_name);
            }
            RenameRule::AddSuffix(s) => {
                new_name = format!("{}{}", new_name, s);
            }
            RenameRule::ChangeCase(t) => {
                new_name = apply_case(&new_name, t);
            }
            RenameRule::RemovePattern(s) => {
                if let Some(re) = regex::Regex::new(s).ok() {
                    new_name = re.replace_all(&new_name, "").to_string();
                } else {
                    new_name = new_name.replace(s, "");
                }
            }
            RenameRule::Numbering {
                start,
                width,
                placeholder,
            } => {
                let num_str = format!("{:0>width$}", start + index, width = *width);
                new_name = new_name.replace(placeholder, &num_str);
            }
        }

        new_name
    }
}

fn apply_case(s: &str, transform: &CaseTransform) -> String {
    match transform {
        CaseTransform::Upper => s.to_uppercase(),
        CaseTransform::Lower => s.to_lowercase(),
        CaseTransform::Title => {
            let mut result = String::new();
            let mut capitalize = true;
            for ch in s.chars() {
                if ch.is_whitespace() || ch == '_' || ch == '-' {
                    result.push(ch);
                    capitalize = true;
                } else if capitalize {
                    result.extend(ch.to_uppercase());
                    capitalize = false;
                } else {
                    result.extend(ch.to_lowercase());
                }
            }
            result
        }
        CaseTransform::Toggle => s
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    c.to_lowercase().to_string()
                } else {
                    c.to_uppercase().to_string()
                }
            })
            .collect(),
    }
}

fn split_filename(filename: &str) -> (String, String) {
    match filename.rfind('.') {
        Some(dot) if dot > 0 => {
            let name = &filename[..dot];
            let ext = &filename[dot + 1..];
            (name.to_string(), ext.to_string())
        }
        _ => (filename.to_string(), String::new()),
    }
}

pub fn apply_rules(filename: &str, rules: &[RenameRule], index: u32) -> String {
    let mut result = filename.to_string();
    for rule in rules {
        result = rule.apply(&result, index);
    }
    result
}

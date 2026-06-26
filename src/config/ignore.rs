use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IgnoreRules {
    ignore_names: Vec<String>,
    ignore_globs: Vec<String>,
    glob_set: GlobSet,
}

impl IgnoreRules {
    pub fn defaults() -> Self {
        Self::from_toml(DEFAULT_IGNORE_TOML).expect("default ignore rules must parse")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read ignore file {}", path.display()))?;
        Self::from_toml(&text)
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        let raw: IgnoreToml = toml::from_str(text)?;
        Self::build(raw.ignore_names, raw.ignore_globs)
    }

    pub fn with_extra_globs(mut self, globs: &[&str]) -> Result<Self> {
        for g in globs {
            self.ignore_globs.push(g.to_string());
        }
        Self::build(self.ignore_names, self.ignore_globs)
    }

    fn build(ignore_names: Vec<String>, ignore_globs: Vec<String>) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for g in &ignore_globs {
            builder.add(Glob::new(g).with_context(|| format!("bad glob: {g}"))?);
        }
        let glob_set = builder.build()?;
        Ok(Self {
            ignore_names,
            ignore_globs,
            glob_set,
        })
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if self.ignore_names.iter().any(|n| n == name) {
                return true;
            }
        }
        self.glob_set.is_match(path)
    }
}

#[derive(serde::Deserialize)]
struct IgnoreToml {
    ignore_names: Vec<String>,
    ignore_globs: Vec<String>,
}

const DEFAULT_IGNORE_TOML: &str = r#"
ignore_names = [
  ".DS_Store", "Thumbs.db", "desktop.ini",
  ".Spotlight-V100", ".Trashes", ".fseventsd",
  ".TemporaryItems", ".DocumentRevisions-V100",
]
ignore_globs = [
  "**/.git/**",
  "**/.svn/**",
  "**/node_modules/**",
  "**/__MACOSX/**",
  "**/.cache/**",
]
"#;

pub fn load_ignore_rules(config_path: Option<&Path>, extra: &[String]) -> Result<IgnoreRules> {
    let mut rules = if let Some(path) = config_path {
        IgnoreRules::load(path)?
    } else {
        let default_path = dirs_config_file();
        if default_path.exists() {
            IgnoreRules::load(&default_path)?
        } else {
            IgnoreRules::defaults()
        }
    };
    if !extra.is_empty() {
        let refs: Vec<&str> = extra.iter().map(String::as_str).collect();
        rules = rules.with_extra_globs(&refs)?;
    }
    Ok(rules)
}

fn dirs_config_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/filetreematch/ignore.toml")
}

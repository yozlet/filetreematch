use crate::db::annotations::{get_annotation, set_annotation};
use crate::db::query::{is_exact_duplicate, list_pairs, SubsetPairRow};
use crate::db::Database;
use anyhow::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Unreviewed,
    DeleteCandidates,
}

impl Filter {
    pub fn next(self) -> Self {
        match self {
            Filter::All => Filter::Unreviewed,
            Filter::Unreviewed => Filter::DeleteCandidates,
            Filter::DeleteCandidates => Filter::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Unreviewed => "unreviewed",
            Filter::DeleteCandidates => "delete candidates",
        }
    }
}

pub struct App {
    pub pairs: Vec<SubsetPairRow>,
    pub selected: usize,
    pub filter: Filter,
    pub search: String,
    pub search_mode: bool,
    pub note_mode: bool,
    pub note_buffer: String,
    pub status_message: String,
    full_detail: bool,
    db: Database,
}

impl App {
    pub fn new(db: Database, full_detail: bool) -> Result<Self> {
        let mut app = Self {
            pairs: Vec::new(),
            selected: 0,
            filter: Filter::All,
            search: String::new(),
            search_mode: false,
            note_mode: false,
            note_buffer: String::new(),
            status_message: String::new(),
            full_detail,
            db,
        };
        app.refresh_pairs()?;
        Ok(app)
    }

    pub fn refresh_pairs(&mut self) -> Result<()> {
        let status_filter = match self.filter {
            Filter::All | Filter::Unreviewed => None,
            Filter::DeleteCandidates => Some("delete_candidate"),
        };

        let mut pairs = list_pairs(self.db.conn(), self.full_detail, status_filter)?;

        if self.filter == Filter::Unreviewed {
            pairs.retain(|pair| {
                match subset_dir_id(self.db.conn(), &pair.subset_path) {
                    Ok(id) => get_annotation(self.db.conn(), id)
                        .map(|a| a.is_none())
                        .unwrap_or(false),
                    Err(_) => false,
                }
            });
        }

        if !self.search.is_empty() {
            let query = self.search.to_lowercase();
            pairs.retain(|pair| {
                pair.subset_path.to_lowercase().contains(&query)
                    || pair.superset_path.to_lowercase().contains(&query)
            });
        }

        self.pairs = pairs;
        if self.selected >= self.pairs.len() {
            self.selected = self.pairs.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn cycle_filter(&mut self) -> Result<()> {
        self.filter = self.filter.next();
        self.refresh_pairs()?;
        self.status_message = format!("Filter: {}", self.filter.label());
        Ok(())
    }

    pub fn mark_selected(&mut self, status: &str) -> Result<()> {
        let Some(pair) = self.pairs.get(self.selected) else {
            self.status_message = "No pair selected".to_string();
            return Ok(());
        };

        let dir_id = subset_dir_id(self.db.conn(), &pair.subset_path)?;
        let notes = get_annotation(self.db.conn(), dir_id)?
            .map(|a| a.notes)
            .unwrap_or_default();
        set_annotation(self.db.conn(), dir_id, status, &notes)?;

        let path = pair.subset_path.clone();
        self.status_message = format!("Marked {path} as {status}");
        Ok(())
    }

    pub fn toggle_note_mode(&mut self) -> Result<()> {
        if self.note_mode {
            self.save_note()?;
        } else {
            self.note_mode = true;
            self.search_mode = false;
            self.note_buffer = if let Some(pair) = self.pairs.get(self.selected) {
                let dir_id = subset_dir_id(self.db.conn(), &pair.subset_path)?;
                get_annotation(self.db.conn(), dir_id)?
                    .map(|a| a.notes)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            self.status_message = "Note mode (Enter save, Esc cancel)".to_string();
        }
        Ok(())
    }

    pub fn save_note(&mut self) -> Result<()> {
        let Some(pair) = self.pairs.get(self.selected) else {
            self.note_mode = false;
            return Ok(());
        };

        let dir_id = subset_dir_id(self.db.conn(), &pair.subset_path)?;
        let status = get_annotation(self.db.conn(), dir_id)?
            .map(|a| a.status)
            .unwrap_or_else(|| "undecided".to_string());
        set_annotation(self.db.conn(), dir_id, &status, &self.note_buffer)?;

        self.note_mode = false;
        self.status_message = "Note saved".to_string();
        Ok(())
    }

    pub fn cancel_note(&mut self) {
        self.note_mode = false;
        self.note_buffer.clear();
        self.status_message = "Note cancelled".to_string();
    }

    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.note_mode = false;
        self.status_message = "Search (Enter apply, Esc clear)".to_string();
    }

    pub fn apply_search(&mut self) -> Result<()> {
        self.search_mode = false;
        self.refresh_pairs()?;
        Ok(())
    }

    pub fn clear_search(&mut self) -> Result<()> {
        self.search = String::new();
        self.search_mode = false;
        self.refresh_pairs()?;
        self.status_message = "Search cleared".to_string();
        Ok(())
    }

    pub fn selected_pair(&self) -> Option<&SubsetPairRow> {
        self.pairs.get(self.selected)
    }

    pub fn annotation_for_selected(&self) -> Result<Option<crate::db::annotations::Annotation>> {
        let Some(pair) = self.selected_pair() else {
            return Ok(None);
        };
        let dir_id = subset_dir_id(self.db.conn(), &pair.subset_path)?;
        get_annotation(self.db.conn(), dir_id)
    }

    pub fn is_selected_exact_duplicate(&self) -> bool {
        let Some(pair) = self.selected_pair() else {
            return false;
        };

        match (
            subset_dir_id(self.db.conn(), &pair.subset_path),
            subset_dir_id(self.db.conn(), &pair.superset_path),
        ) {
            (Ok(subset_id), Ok(superset_id)) => {
                is_exact_duplicate(self.db.conn(), subset_id, superset_id).unwrap_or(false)
            }
            _ => false,
        }
    }

    pub fn subset_annotation_marker(&self, subset_path: &str) -> String {
        match subset_dir_id(self.db.conn(), subset_path) {
            Ok(id) => match get_annotation(self.db.conn(), id) {
                Ok(Some(annotation)) => match annotation.status.as_str() {
                    "keep" => " [K]".to_string(),
                    "delete_candidate" => " [D]".to_string(),
                    _ => " [U]".to_string(),
                },
                _ => String::new(),
            },
            Err(_) => String::new(),
        }
    }
}

fn subset_dir_id(conn: &rusqlite::Connection, path: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT id FROM directories WHERE full_path = ?1",
        [path],
        |row| row.get(0),
    )?)
}

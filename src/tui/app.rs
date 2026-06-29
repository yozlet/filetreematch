use crate::db::annotations::{load_all_annotations, set_annotation, Annotation};
use crate::db::query::{list_pairs, load_path_index};
use crate::db::Database;
use crate::tui::display::{build_rows, selection_detail, SelectionDetail, TuiRow};
use anyhow::Result;
use std::collections::HashMap;

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
    pub rows: Vec<TuiRow>,
    pub selected: usize,
    pub filter: Filter,
    pub search: String,
    pub search_mode: bool,
    pub note_mode: bool,
    pub note_buffer: String,
    pub status_message: String,
    pub selection_detail: Option<SelectionDetail>,
    full_detail: bool,
    path_to_id: HashMap<String, i64>,
    annotations: HashMap<i64, Annotation>,
    db: Database,
}

impl App {
    pub fn new(db: Database, full_detail: bool) -> Result<Self> {
        let mut app = Self {
            rows: Vec::new(),
            selected: 0,
            filter: Filter::All,
            search: String::new(),
            search_mode: false,
            note_mode: false,
            note_buffer: String::new(),
            status_message: String::new(),
            selection_detail: None,
            full_detail,
            path_to_id: HashMap::new(),
            annotations: HashMap::new(),
            db,
        };
        app.refresh_pairs()?;
        Ok(app)
    }

    pub fn refresh_pairs(&mut self) -> Result<()> {
        let conn = self.db.conn();
        self.path_to_id = load_path_index(conn)?;
        self.annotations = load_all_annotations(conn)?;

        let status_filter = match self.filter {
            Filter::All | Filter::Unreviewed => None,
            Filter::DeleteCandidates => Some("delete_candidate"),
        };

        let mut pairs = list_pairs(conn, self.full_detail, status_filter)?;

        if self.filter == Filter::Unreviewed {
            pairs.retain(|pair| {
                self.path_to_id
                    .get(&pair.subset_path)
                    .and_then(|id| self.annotations.get(id))
                    .is_none()
            });
        }

        if !self.search.is_empty() {
            let query = self.search.to_lowercase();
            pairs.retain(|pair| {
                pair.subset_path.to_lowercase().contains(&query)
                    || pair.superset_path.to_lowercase().contains(&query)
            });
        }

        self.rows = build_rows(&pairs, &self.path_to_id, &self.annotations);
        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
        self.refresh_selection_detail();
        Ok(())
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.refresh_selection_detail();
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(self.rows.len().saturating_sub(1));
        self.refresh_selection_detail();
    }

    fn refresh_selection_detail(&mut self) {
        self.selection_detail = self
            .rows
            .get(self.selected)
            .map(|row| selection_detail(row, &self.path_to_id, &self.annotations));
    }

    pub fn cycle_filter(&mut self) -> Result<()> {
        self.filter = self.filter.next();
        self.refresh_pairs()?;
        self.status_message = format!("Filter: {}", self.filter.label());
        Ok(())
    }

    pub fn mark_selected(&mut self, status: &str) -> Result<()> {
        let path = self
            .rows
            .get(self.selected)
            .map(|row| row.subset_path.clone());
        let Some(path) = path else {
            self.status_message = "No pair selected".to_string();
            return Ok(());
        };

        let dir_id = subset_dir_id(&self.path_to_id, &path)?;
        let notes = self
            .annotations
            .get(&dir_id)
            .map(|a| a.notes.clone())
            .unwrap_or_default();
        set_annotation(self.db.conn(), dir_id, status, &notes)?;
        self.annotations.insert(
            dir_id,
            Annotation {
                status: status.to_string(),
                notes,
            },
        );
        self.refresh_pairs()?;

        self.status_message = format!("Marked {path} as {status}");
        Ok(())
    }

    pub fn toggle_note_mode(&mut self) -> Result<()> {
        if self.note_mode {
            self.save_note()?;
        } else {
            self.note_mode = true;
            self.search_mode = false;
            self.note_buffer = self
                .selection_detail
                .as_ref()
                .map(|d| d.notes.clone())
                .unwrap_or_default();
            self.status_message = "Note mode (Enter save, Esc cancel)".to_string();
        }
        Ok(())
    }

    pub fn save_note(&mut self) -> Result<()> {
        let Some(row) = self.rows.get(self.selected) else {
            self.note_mode = false;
            return Ok(());
        };

        let dir_id = subset_dir_id(&self.path_to_id, &row.subset_path)?;
        let status = self
            .annotations
            .get(&dir_id)
            .map(|a| a.status.as_str())
            .unwrap_or("undecided");
        set_annotation(self.db.conn(), dir_id, status, &self.note_buffer)?;
        self.annotations.insert(
            dir_id,
            Annotation {
                status: status.to_string(),
                notes: self.note_buffer.clone(),
            },
        );
        self.refresh_selection_detail();

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

    pub fn selected_row(&self) -> Option<&TuiRow> {
        self.rows.get(self.selected)
    }
}

fn subset_dir_id(path_to_id: &HashMap<String, i64>, path: &str) -> Result<i64> {
    path_to_id
        .get(path)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("directory not found: {path}"))
}

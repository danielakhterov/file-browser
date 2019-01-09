use cursive::{
    align::Align,
    event::{Event, EventResult, Key},
    vec::Vec2,
    view::View,
    Printer,
};
use failure::Error;
use std::{cell::Cell, cmp, fs::read_dir, path::Path, rc::Rc, result::Result};
#[macro_use]
use crate::print_full_width_with_selection;
use crate::{color_pair::ColorPair, entry::Entry, print_full_width};
use number_prefix::{binary_prefix, Prefixed, Standalone};
use std::{fs::read_link, path::PathBuf};
use core::convert::TryFrom;
use failure::bail;

pub(crate) struct DirectoryView {
    pub(crate) path: PathBuf,
    pub(crate) dirs: Vec<Entry>,
    pub(crate) files: Vec<Entry>,
    focus: Rc<Cell<usize>>,
    align: Align,
    last_offset: Cell<usize>,
}

impl DirectoryView {
    fn new(path: PathBuf) -> Self {
        DirectoryView {
            path,
            dirs: Vec::new(),
            files: Vec::new(),
            focus: Rc::new(Cell::new(0)),
            align: Align::top_left(),
            last_offset: Cell::new(0 as usize),
        }
    }

    pub(crate) fn focus_path(&mut self, path: PathBuf) {
        for (i, entry) in self.dirs.iter().enumerate() {
            if entry.path == path {
                self.focus.set(i);
            }
        }

        for (i, entry) in self.files.iter().enumerate() {
            if entry.path == path {
                self.focus.set(i + self.dirs.len());
            }
        }
    }

    fn size(entry: PathBuf) -> String {
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => return "Broken Link".to_string(),
        };

        let filetype = meta.file_type();

        if filetype.is_dir() {
            let dir = match read_dir(entry) {
                Ok(dir) => dir,
                Err(_) => return "?".to_string(),
            };

            dir.into_iter()
                .filter(Result::is_ok)
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .len().to_string()
        } else if filetype.is_file() {
            match binary_prefix(meta.len() as f64) {
                Standalone(bytes) => format!("{} B", bytes),
                Prefixed(prefix, n) => format!("{:.0} {}B", n, prefix),
            }
        } else if filetype.is_symlink() {
            match read_link(entry) {
                Ok(link) => DirectoryView::size(link),
                Err(_) => "Broken Link".to_string(),
            }
        } else {
            "Error".to_string()
        }
    }

    pub(crate) fn focus(&self) -> usize {
        self.focus.get()
    }

    pub(crate) fn change_focus_by(&mut self, difference: i64) {
        let focus = self.focus.get();
        let new_focus = if difference > 0 {
            if focus + difference as usize >= self.total_list_size() {
                (self.total_list_size() - 1) as usize
            } else {
                focus.saturating_add(difference as usize)
            }
        } else if difference < 0 {
            focus.saturating_sub(difference.abs() as usize)
        } else {
            focus
        };
        self.focus.set(new_focus);
    }

    pub(crate) fn total_list_size(&self) -> usize {
        self.dirs.len() + self.files.len()
    }
}

impl TryFrom<PathBuf> for DirectoryView {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let mut view = DirectoryView::new(path.clone());

        for entry in read_dir(path.as_path())?
            .into_iter()
            .filter(Result::is_ok)
            .map(Result::unwrap)
        {
            let path = entry.path();

            let name = entry.file_name().into_string();
            if name.is_err() {
                continue;
            }

            let name = name.unwrap();
            let meta = entry.metadata()?;
            let filetype = meta.file_type();

            let size = DirectoryView::size(entry.path());
            let size = if filetype.is_symlink() {
                format!("-> {}", size)
            } else {
                size
            };

            let color = ColorPair::new(&entry).unwrap_or_else(|_| ColorPair::default());

            match meta.is_dir() {
                true => &mut view.dirs,
                false => &mut view.files,
            }
            .push(Entry {
                path,
                name,
                size,
                color,
            });
        }

        view.dirs.sort();
        view.files.sort();

        Ok(view)
    }
}

impl View for DirectoryView {
    fn draw(&self, printer: &Printer) {
        let height = self.dirs.len() + self.files.len();
        let offset = self.align.v.get_offset(height, printer.size.y);
        let printer = &printer.offset((0, offset));
        let focus = self.focus();

        // Which element should we start at to make sure the focused element
        // is in view.
        let start = if self.last_offset.get() > focus {
            focus
        } else if self.last_offset.get() + printer.size.y - 1 < focus {
            focus - printer.size.y + 1
        } else {
            self.last_offset.get()
        };

        // Set the current start as the next offset
        self.last_offset.set(start);

        // Loop through all the lines in the printer
        // Either print a file at the current line or a directory
        // Based on the current focus
        for i in 0..printer.size.y {
            let element = i + start;
            if element < self.dirs.len() {
                let name = &self.dirs[element].name;
                let color = &self.dirs[element].color;
                let size = &self.dirs[element].size;
                print_full_width_with_selection!(printer, element, focus, name, size, color, i);
            } else if element - self.dirs.len() < self.files.len() {
                let name = &self.files[element - self.dirs.len()].name;
                let color = &self.files[element - self.dirs.len()].color;
                let size = &self.files[element - self.dirs.len()].size;
                print_full_width_with_selection!(printer, element, focus, name, size, color, i);
            }
        }
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        let h = self.dirs.len() + self.files.len();

        let w = {
            cmp::max(
                self.dirs
                    .iter()
                    .map(|dir| dir.name.len())
                    .max()
                    .unwrap_or(1),
                self.files
                    .iter()
                    .map(|file| file.name.len())
                    .max()
                    .unwrap_or(1),
            )
        };

        Vec2::new(w, h)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) => self.change_focus_by(-1),
            Event::Key(Key::Down) => self.change_focus_by(1),
            Event::Key(Key::PageUp) => self.change_focus_by(-10),
            Event::Key(Key::PageDown) => self.change_focus_by(10),
            Event::Key(Key::Home) => self.focus.set(0),
            Event::Key(Key::End) => self.focus.set(self.total_list_size() - 1),
            Event::Char(c) => match c {
                'j' => self.change_focus_by(1),
                'k' => self.change_focus_by(-1),
                _ => return EventResult::Ignored,
            },
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }
}

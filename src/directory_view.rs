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
use crate::print_full_width;
use crate::{color_pair::ColorPair, entry::Entry};
use config::Config;

pub(crate) struct DirectoryView {
    dirs: Vec<Entry>,
    files: Vec<Entry>,
    focus: Rc<Cell<usize>>,
    align: Align,
    last_offset: Cell<usize>,
}

impl DirectoryView {
    fn new() -> Self {
        DirectoryView {
            dirs: Vec::new(),
            files: Vec::new(),
            focus: Rc::new(Cell::new(0)),
            align: Align::top_left(),
            last_offset: Cell::new(0 as usize),
        }
    }

    pub fn from(path: &Path, settings: &mut Config) -> Result<DirectoryView, Error> {
        let mut view = DirectoryView::new();

        for entry in read_dir(path)?
            .into_iter()
            .filter(Result::is_ok)
            .map(Result::unwrap)
        {
            let name = entry.file_name().into_string();
            match name {
                Ok(_) => {}
                Err(_) => continue,
            }

            let name = name.unwrap();

            let meta = entry.metadata()?;

            let size = if meta.is_dir() {
                read_dir(&Path::new(&entry.path()))?
                    .into_iter()
                    .filter(Result::is_ok)
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
                    .len() as usize
            } else if meta.is_file() {
                meta.len() as usize
            } else {
                0 as usize
            };

            match meta.is_dir() {
                true => &mut view.dirs,
                false => &mut view.files,
            }
            .push(Entry {
                name,
                size,
                color: ColorPair::new(&entry, settings),
            });
        }

        view.dirs.sort();
        view.files.sort();

        Ok(view)
    }

    pub fn focus_up(&mut self) {
        if self.focus() > 0 {
            self.focus.set(self.focus.get().saturating_sub(1))
        }
    }

    pub fn focus_down(&mut self) {
        if self.focus() < (self.total_list_size() - 1) {
            self.focus.set(self.focus.get().saturating_add(1))
        }
    }

    pub fn focus_first(&mut self) {
        self.focus.set(0);
    }

    pub fn focus_last(&mut self) {
        self.focus.set(self.total_list_size() - 1)
    }

    pub fn focus(&self) -> usize {
        self.focus.get()
    }

    pub fn change_focus_by(&mut self, difference: i64) {
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

    pub fn total_list_size(&self) -> usize {
        self.dirs.len() + self.files.len()
    }
}

impl View for DirectoryView {
    fn draw(&self, printer: &Printer) {
        let h = self.dirs.len() + self.files.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset));
        let dirs_len = self.dirs.len();
        let focus = self.focus();

        let start = if self.last_offset.get() > focus {
            focus
        } else if self.last_offset.get() + printer.size.y - 1 < focus {
            focus - printer.size.y + 1
        } else {
            self.last_offset.get()
        };

        let end = if self.dirs.len() + self.files.len() < printer.size.y - 1 {
            self.dirs.len() + self.files.len() + 1
        } else {
            printer.size.y
        };

        self.last_offset.set(start);

        for i in 0..printer.size.y {
            let element = i + start;
            if element < self.dirs.len() {
                let name = &self.dirs[element].name;
                let color = &self.dirs[element].color;

                if element == self.focus() {
                    printer.with_color(color.highlight, print_full_width!(name, i));
                } else {
                    printer.with_color(color.regular, print_full_width!(name, i));
                }
            } else if element - self.dirs.len() < self.files.len() {
                let name = &self.files[element - self.dirs.len()].name;
                let color = &self.files[element - self.dirs.len()].color;

                if element == self.focus() {
                    printer.with_color(color.highlight, print_full_width!(name, i));
                } else {
                    printer.with_color(color.regular, print_full_width!(name, i));
                }
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
            Event::Key(Key::Up) => self.focus_down(),
            Event::Key(Key::Down) => self.focus_up(),
            // Event::Key(Key::PageUp) => self.change_focus_by(10),
            // Event::Key(Key::PageDown) => self.change_focus_by(-10),
            Event::Key(Key::Home) => self.focus.set(0),
            Event::Key(Key::End) => self.focus.set(self.total_list_size() - 1),
            Event::Char(c) => match c {
                'j' => self.focus_down(),
                'k' => self.focus_up(),
                _ => return EventResult::Ignored,
            },
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }
}
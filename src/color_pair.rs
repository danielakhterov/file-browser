use config::Config;
use cursive::theme::{BaseColor, Color, ColorStyle};
use std::{fs::DirEntry, ops::BitAnd, os::unix::fs::PermissionsExt};

pub struct ColorPair {
    pub regular: ColorStyle,
    pub highlight: ColorStyle,
}

impl Default for ColorPair {
    fn default() -> Self {
        ColorPair {
            regular: ColorStyle::primary(),
            highlight: ColorStyle::highlight(),
        }
    }
}

impl ColorPair {
    pub fn new(entry: &DirEntry, settings: &mut Config) -> ColorPair {
        let meta = entry.metadata().unwrap();
        let filetype = match entry.file_type() {
            Ok(filetype) => filetype,
            Err(_) => return ColorPair::default(),
        };

        if filetype.is_dir() {
            return ColorPair {
                regular: ColorStyle::new(
                    Color::Dark(BaseColor::Blue),
                    Color::Dark(BaseColor::Black),
                ),
                highlight: ColorStyle::new(
                    Color::Dark(BaseColor::Black),
                    Color::Dark(BaseColor::Blue),
                ),
            };
        } else if filetype.is_file() {
            if meta.permissions().mode().bitand(1) == 1 {
                return ColorPair {
                    regular: ColorStyle::new(
                        Color::Dark(BaseColor::Green),
                        Color::Dark(BaseColor::Black),
                    ),
                    highlight: ColorStyle::new(
                        Color::Dark(BaseColor::Black),
                        Color::Dark(BaseColor::Green),
                    ),
                };
            }

            let ext = entry.path();
            let ext = ext.extension();
            if ext.is_none() {
                return ColorPair::default();
            }

            let ext = ext.unwrap().to_str();
            if let Some(ext) = ext {
                match settings.get_str(&ext) {
                    Ok(s) => match Color::parse(&s) {
                        Some(color) => {
                            return ColorPair {
                                regular: ColorStyle::new(color, Color::Dark(BaseColor::Black)),
                                highlight: ColorStyle::new(Color::Dark(BaseColor::Black), color),
                            };
                        }
                        None => {}
                    },
                    Err(_) => {}
                }
            }
            return ColorPair {
                regular: ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Black),
                ),
                highlight: ColorStyle::new(
                    Color::Dark(BaseColor::Black),
                    Color::Dark(BaseColor::White),
                ),
            };
        } else if filetype.is_symlink() {
            return ColorPair {
                regular: ColorStyle::new(
                    Color::Dark(BaseColor::Cyan),
                    Color::Dark(BaseColor::Black),
                ),
                highlight: ColorStyle::new(
                    Color::Dark(BaseColor::Black),
                    Color::Dark(BaseColor::Cyan),
                ),
            };
        } else {
            return ColorPair::default();
        }
    }
}

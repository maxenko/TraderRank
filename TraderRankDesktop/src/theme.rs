#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn toggle(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }
}

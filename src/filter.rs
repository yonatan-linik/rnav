#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    In,
    Out,
}

pub struct Filter {
    kind: FilterKind,
    regex: regex::Regex,
    enabled: bool,
}

impl Filter {
    pub fn new(kind: FilterKind, regex: regex::Regex) -> Self {
        Self {
            kind,
            regex,
            enabled: true,
        }
    }

    pub fn keep_line(&self, line: &str) -> bool {
        if !self.enabled {
            return true;
        }

        match self.kind {
            FilterKind::In => self.regex.is_match(line),
            FilterKind::Out => !self.regex.is_match(line),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn filter_kind(&self) -> FilterKind {
        self.kind
    }
}

use std::path::PathBuf;

pub struct IgnoreFilter {
    dir_list: Vec<PathBuf>,
}

impl IgnoreFilter {
    pub fn new(dir_list: Vec<PathBuf>) -> Self {
        Self { dir_list }
    }
    pub fn new_str(dir_list: Vec<String>) -> Self {
        Self {
            dir_list: dir_list
                .iter()
                .map(move |path| PathBuf::from(path))
                .collect(),
        }
    }

    pub fn filter(&self) -> Vec<PathBuf> {
        // TODO: filter
        self.dir_list.clone()
    }
}

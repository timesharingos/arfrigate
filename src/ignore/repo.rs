use std::{fs, path::Path, vec};

use crate::ignore::tree::{self, IgnoreTreeNode};

pub struct RepoFilter {
    root: String,
    filelist: Vec<String>,
}

impl RepoFilter {
    #[allow(dead_code)]
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn filelist(&self) -> &Vec<String> {
        &self.filelist
    }
}

impl RepoFilter {
    pub fn new<P>(target: P) -> Option<Self>
    where
        P: AsRef<Path>,
    {
        Self::filter(target.as_ref()).map(|list| Self {
            root: target
                .as_ref()
                .as_os_str()
                .to_str()
                .expect("illegal UTF-8 code")
                .to_string(),
            filelist: list,
        })
    }

    fn filter<P>(target: P) -> Option<Vec<String>>
    where
        P: AsRef<Path>,
    {
        if fs::exists(target.as_ref().join(".gitignore")).is_ok_and(|x| x)
            || fs::exists(target.as_ref().join(".git")).is_ok_and(|x| x)
        {
            return Some(Self::process_gitignore(target));
        }
        let mut result = vec![];
        for entry in fs::read_dir(target.as_ref()).ok()? {
            match entry {
                Err(_) => continue,
                Ok(entry) => {
                    let entry = target.as_ref().join(entry.file_name());
                    if entry.is_file() {
                        result.push(
                            entry
                                .as_os_str()
                                .to_str()
                                .expect("illegal UTF-8 code")
                                .to_string(),
                        );
                    } else {
                        result.append(
                            &mut Self::filter(
                                target
                                    .as_ref()
                                    .join(entry.file_name().expect("unexpected relative path")),
                            )
                            .unwrap_or(vec![]),
                        );
                    }
                }
            }
        }
        Some(result)
    }

    fn process_gitignore<P>(target: P) -> Vec<String>
    where
        P: AsRef<Path>,
    {
        let tree_pattern = IgnoreTreeNode::from_path(target.as_ref().join(".gitignore"));
        match tree_pattern {
            Err(_) => vec![
                target
                    .as_ref()
                    .to_str()
                    .expect("illegal UTF-8 code")
                    .to_string(),
            ],
            Ok(tree_pattern) => Self::process_gitignore0(target, "", &tree_pattern),
        }
    }

    fn process_gitignore0<P, T>(prefix: P, target: T, ignore: &IgnoreTreeNode) -> Vec<String>
    where
        P: AsRef<Path>,
        T: AsRef<Path>,
    {
        let mut result = vec![];
        let real_target = prefix.as_ref().join(target.as_ref());
        for entry in fs::read_dir(real_target.as_path()).expect("non-existing file") {
            match entry {
                Err(_) => continue,
                Ok(entry) => {
                    let entry = target
                        .as_ref()
                        .join(entry.path().file_name().expect("unexpected relative path"));
                    let match_hint =
                        ignore.match_hint(entry.as_os_str().to_str().expect("illegal UTF-8 code"));
                    match match_hint {
                        tree::IgnoreTreeMatchHint::NoneMatch => result.push(
                            prefix
                                .as_ref()
                                .join(entry)
                                .to_str()
                                .expect("illegal UTF-8 code")
                                .to_string(),
                        ),
                        tree::IgnoreTreeMatchHint::BlackOnly => result.push(
                            prefix
                                .as_ref()
                                .join(entry)
                                .to_str()
                                .expect("illegal UTF-8 code")
                                .to_string(),
                        ),
                        tree::IgnoreTreeMatchHint::WhiteOnly => continue,
                        tree::IgnoreTreeMatchHint::Sub => {
                            result.append(&mut Self::process_gitignore0(
                                prefix.as_ref(),
                                entry,
                                ignore,
                            ));
                        }
                    }
                }
            }
        }
        result
    }
}

#[derive(Default)]
pub struct FilterList {
    origin_list: Vec<String>,
    actual_list: Vec<String>,
}

impl FilterList {
    pub fn new(origin_list: Vec<String>) -> Self {
        let actual_list = Self::filter_dirs(&origin_list);
        Self {
            origin_list,
            actual_list,
        }
    }

    fn filter_dirs(target_dirs: &[String]) -> Vec<String> {
        target_dirs
            .iter()
            .map(RepoFilter::new)
            .filter(|actual| actual.is_some())
            .map(|actual| actual.unwrap().filelist().to_owned())
            .flat_map(|actual| actual.into_iter())
            .collect()
    }

    #[allow(dead_code)]
    pub fn add_origin(&mut self, target: String) {
        let mut new_actual = Self::filter_dirs(std::slice::from_ref(&target));
        self.actual_list.append(&mut new_actual);
        self.origin_list.push(target);
    }

    #[allow(dead_code)]
    pub fn get_origin(&self) -> &Vec<String> {
        &self.origin_list
    }
    pub fn get_actual(&self) -> &Vec<String> {
        &self.actual_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_basic_filter() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let root = temp_dir.path();

        let gitignore_path = root.join(".gitignore");
        let mut file = File::create(&gitignore_path).unwrap();
        writeln!(file, "target").unwrap();
        writeln!(file, "*.log").unwrap();

        fs::create_dir(root.join("src")).unwrap();
        File::create(root.join("src").join("main.rs")).unwrap();
        File::create(root.join("Cargo.toml")).unwrap();
        File::create(root.join("debug.log")).unwrap();

        let filter = RepoFilter::new(root).expect("should create filter");
        assert!(
            filter
                .filelist()
                .contains(&root.join("src").to_string_lossy().to_string())
        );
        assert!(
            !filter
                .filelist()
                .contains(&root.join("debug.log").to_string_lossy().to_string())
        );
    }
}

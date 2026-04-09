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
    // keep Option<_> only for compatibility
    fn filter<P>(target: P) -> Option<Vec<String>>
    where
        P: AsRef<Path>,
    {
        match Self::filter0(target.as_ref(), "") {
            None => Some(vec![target.as_ref().to_string_lossy().to_string()]),
            Some(list) => Some(list),
        }
    }

    fn filter0<P, T>(prefix: P, target: T) -> Option<Vec<String>>
    where
        P: AsRef<Path>,
        T: AsRef<Path>,
    {
        let real_target = prefix.as_ref().join(target.as_ref());
        if fs::exists(real_target.join(".gitignore")).is_ok_and(|x| x)
            || fs::exists(real_target.join(".git")).is_ok_and(|x| x)
        {
            return Some(Self::process_gitignore(real_target));
        }
        let entries = fs::read_dir(real_target.as_path()).ok();
        match entries {
            None => Some(vec![]),
            Some(entries) => {
                let mut filtered = false;
                let mut pending_selected_dir = vec![];
                for entry in entries {
                    match entry {
                        Err(e) => {
                            println!("{}", e);
                            filtered = true;
                            continue;
                        }
                        Ok(entry) => {
                            let real_entry = real_target.join(entry.file_name());
                            let entry = target.as_ref().join(entry.file_name());
                            if real_entry.is_file() {
                                pending_selected_dir.push(
                                    real_entry
                                        .as_os_str()
                                        .to_str()
                                        .expect("illegal UTF-8 code")
                                        .to_string(),
                                );
                            } else {
                                let sub_result = Self::filter0(prefix.as_ref(), entry);
                                match sub_result {
                                    None => pending_selected_dir.push(
                                        real_entry
                                            .as_os_str()
                                            .to_str()
                                            .expect("illegal UTF-8 code")
                                            .to_string(),
                                    ),
                                    Some(mut list) => {
                                        pending_selected_dir.append(&mut list);
                                        filtered = true;
                                    }
                                };
                            }
                        }
                    }
                }
                if filtered {
                    Some(pending_selected_dir)
                } else {
                    None
                }
            }
        }
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
                        tree::IgnoreTreeMatchHint::WildcardAllMatch => {
                            // wildcard detected, check each subdirectory
                            match Self::process_traverse(prefix.as_ref(), entry.as_path(), ignore) {
                                None => result.push(
                                    prefix
                                        .as_ref()
                                        .join(entry)
                                        .to_str()
                                        .expect("illegal UTF-8 code")
                                        .to_string(),
                                ),
                                Some(mut list) => result.append(&mut list),
                            };
                        }
                    }
                }
            }
        }
        result
    }

    fn process_traverse<P, T>(prefix: P, target: T, ignore: &IgnoreTreeNode) -> Option<Vec<String>>
    where
        P: AsRef<Path>,
        T: AsRef<Path>,
    {
        let real_target = prefix.as_ref().join(target.as_ref());
        if real_target.is_file() {
            if ignore.match_pattern(
                target
                    .as_ref()
                    .as_os_str()
                    .to_str()
                    .expect("illegal UTF-8 code")
                    .replace("\\", "/"),
            ) {
                return Some(vec![]);
            } else {
                return None;
            }
        }
        let mut pending_dir_filter = vec![];
        if ignore.match_pattern(
            target
                .as_ref()
                .as_os_str()
                .to_str()
                .expect("illegal UTF-8 code")
                .replace("\\", "/"),
        ) {
            pending_dir_filter.push(
                real_target
                    .as_os_str()
                    .to_str()
                    .expect("illegal UTF-8 code")
                    .to_string(),
            );
        }
        let dir_iter = fs::read_dir(real_target.clone());
        match dir_iter {
            Err(_) => None,
            Ok(entries) => match entries.fold(None, |cur: Option<Vec<String>>, e| match e {
                Err(_) => cur,
                Ok(entry) => {
                    let path = target.as_ref().join(entry.file_name());
                    let real_path = real_target.join(entry.file_name());
                    let mut pending_filter_result = vec![];
                    let mut pending_selected_result = vec![];
                    if real_path.is_file() {
                        if ignore.match_pattern(
                            path.as_os_str()
                                .to_str()
                                .expect("illegal UTF-8 code")
                                .replace("\\", "/"),
                        ) {
                            pending_filter_result.push(
                                real_path
                                    .as_os_str()
                                    .to_str()
                                    .expect("illegal UTF-8 code")
                                    .to_string(),
                            );
                        } else {
                            pending_selected_result.push(
                                real_path
                                    .as_os_str()
                                    .to_str()
                                    .expect("illegal UTF-8 code")
                                    .to_string(),
                            );
                        }
                    } else {
                        match Self::process_traverse(prefix.as_ref(), path, ignore) {
                            Some(mut list) => {
                                pending_selected_result.append(&mut list);
                                pending_filter_result.push(
                                    real_path
                                        .as_os_str()
                                        .to_str()
                                        .expect("illegal UTF-8 code")
                                        .to_string(),
                                );
                            }
                            None => pending_selected_result.push(
                                real_path
                                    .as_os_str()
                                    .to_str()
                                    .expect("illegal UTF-8 code")
                                    .to_string(),
                            ),
                        }
                    }
                    if pending_filter_result.is_empty() {
                        match cur {
                            None => None,
                            Some(mut list) => {
                                list.push(
                                    real_path
                                        .as_os_str()
                                        .to_str()
                                        .expect("illegal UTF-8 code")
                                        .to_string(),
                                );
                                Some(list)
                            }
                        }
                    } else {
                        match cur {
                            None => Some(pending_selected_result),
                            Some(mut list) => {
                                list.append(&mut pending_selected_result);
                                Some(list)
                            }
                        }
                    }
                }
            }) {
                None => {
                    if pending_dir_filter.is_empty() {
                        None
                    } else {
                        Some(vec![])
                    }
                }

                Some(list) => Some(list),
            },
        }
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

    #[test]
    fn test_merge_dir() {
        let temp_dir = tempdir().expect("failed to create tmp dir");
        let root = temp_dir.path();

        fs::create_dir(root.join("normal")).unwrap();
        fs::create_dir(root.join("normal").join("dir")).unwrap();
        File::create(root.join("normal").join("file")).unwrap();
        fs::create_dir(root.join("normal").join("dir").join("empty")).unwrap();
        fs::create_dir(root.join("normal").join("dir").join("internal")).unwrap();
        File::create(
            root.join("normal")
                .join("dir")
                .join("internal")
                .join("regular"),
        )
        .unwrap();

        let filter = RepoFilter::new(root).expect("should create filter");
        assert_eq!(filter.filelist().len(), 1);
        assert!(
            filter
                .filelist()
                .contains(&root.to_string_lossy().to_string())
        );
    }
}

use std::{fs, path::Path, vec};

use crate::ignore::tree::{self, IgnoreTreeNode};

pub struct RepoFilter {
    root: String,
    filelist: Vec<String>,
}

impl RepoFilter {
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
                    let entry = target.as_ref().join(entry.path());
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
                            &mut Self::filter(target.as_ref().join(entry)).unwrap_or(vec![]),
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
                    let entry = target.as_ref().join(entry.path());
                    let match_hint =
                        ignore.match_hint(entry.as_os_str().to_str().expect("illegal UTF-8 code"));
                    println!("{:?}/{:?}", entry, match_hint);
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

        println!("{:?}", filter.filelist());

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

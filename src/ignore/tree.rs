use std::{
    convert::AsRef,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use regex::Regex;

pub struct IgnoreMatchPattern {
    original_pattern: String,
    regex: Regex,
}

impl From<String> for IgnoreMatchPattern {
    fn from(value: String) -> Self {
        let str_value: &str = &value;
        str_value.into()
    }
}
impl From<&str> for IgnoreMatchPattern {
    fn from(value: &str) -> Self {
        let regex_str = format!("^{}$", regex::escape(value).replace("\\*", ".*"));
        let regex = Regex::new(&regex_str).unwrap();
        Self {
            original_pattern: value.to_string(),
            regex,
        }
    }
}

impl From<IgnoreMatchPattern> for String {
    fn from(value: IgnoreMatchPattern) -> Self {
        value.original_pattern
    }
}

impl IgnoreMatchPattern {
    pub fn new(pattern_part: String) -> Self {
        pattern_part.into()
    }
    pub fn get_pattern(&self) -> &Regex {
        &self.regex
    }
    pub fn match_pattern<P>(&self, target: P) -> bool
    where
        P: AsRef<str>,
    {
        self.regex.is_match(target.as_ref())
    }
    pub fn get_original_pattern(&self) -> &str {
        &self.original_pattern
    }
}

pub enum IgnoreTreePattern {
    WildCardAll,
    Regular(IgnoreMatchPattern),
}

impl IgnoreTreePattern {
    pub fn match_pattern<P>(&self, target: P) -> bool
    where
        P: AsRef<str>,
    {
        if let Self::Regular(regular) = self {
            regular.match_pattern(target)
        } else {
            true
        }
    }
}

pub struct IgnoreTreeNode {
    ruleset: Vec<(IgnoreTreePattern, Option<IgnoreTreeNode>)>,
    exclude: Vec<(IgnoreTreePattern, Option<IgnoreTreeNode>)>,
}

impl From<String> for IgnoreTreeNode {
    fn from(value: String) -> Self {
        let str_value: &str = &value;
        str_value.into()
    }
}

impl From<&str> for IgnoreTreeNode {
    fn from(value: &str) -> Self {
        let mut ruleset = vec![];
        let mut exclude = vec![];
        let whitelist;
        let pattern: &str;
        let current_pattern;
        match value.strip_prefix("!") {
            Some(stripped) => {
                whitelist = false;
                pattern = stripped;
            }
            None => {
                whitelist = true;
                pattern = value;
            }
        }
        let components = pattern.split_once("/");
        match components {
            None => {
                if pattern == "**" {
                    current_pattern = (IgnoreTreePattern::WildCardAll, None);
                } else {
                    current_pattern = (IgnoreTreePattern::Regular(pattern.into()), None);
                }
            }
            Some((prefix, suffix)) => {
                let remaining_child: IgnoreTreeNode = suffix.into();
                if prefix == "**" {
                    current_pattern = (IgnoreTreePattern::WildCardAll, Some(remaining_child));
                } else {
                    current_pattern = (
                        IgnoreTreePattern::Regular(prefix.into()),
                        Some(remaining_child),
                    );
                }
            }
        }

        if whitelist {
            ruleset.push(current_pattern);
        } else {
            exclude.push(current_pattern);
        }

        Self { ruleset, exclude }
    }
}

impl Default for IgnoreTreeNode {
    fn default() -> Self {
        Self::new()
    }
}

impl IgnoreTreeNode {
    pub fn new() -> Self {
        Self {
            ruleset: vec![],
            exclude: vec![],
        }
    }

    pub fn add_path<P>(&mut self, target: P)
    where
        P: AsRef<str>,
    {
        if target.as_ref() == "" {
            return;
        }
        let actual_target;
        let black;
        if target.as_ref().starts_with("!") {
            black = true;
            actual_target = &target.as_ref()[1..];
        } else {
            black = false;
            actual_target = target.as_ref();
        }
        let target_list = if black {
            &mut self.exclude
        } else {
            &mut self.ruleset
        };
        let split_result = actual_target.split_once("/");
        match split_result {
            Some((prefix, suffix)) => {
                //multi level
                for elem in target_list.iter_mut() {
                    if let Some(child) = elem.1.as_mut() {
                        match &elem.0 {
                            IgnoreTreePattern::WildCardAll => {
                                if prefix == "**" {
                                    child.add_path(suffix);
                                    return;
                                }
                            }
                            IgnoreTreePattern::Regular(regular) => {
                                if regular.get_original_pattern() == prefix {
                                    child.add_path(suffix);
                                    return;
                                }
                            }
                        }
                    }
                }
                let new_child = if prefix == "**" {
                    (IgnoreTreePattern::WildCardAll, Some(suffix.into()))
                } else {
                    (
                        IgnoreTreePattern::Regular(prefix.into()),
                        Some(suffix.into()),
                    )
                };
                target_list.push(new_child);
            }
            None => {
                //single level
                for elem in target_list.iter() {
                    if elem.1.is_none() {
                        match &elem.0 {
                            IgnoreTreePattern::WildCardAll => {
                                if actual_target == "**" {
                                    return;
                                }
                            }
                            IgnoreTreePattern::Regular(regular) => {
                                if regular.get_original_pattern() == actual_target {
                                    return;
                                }
                            }
                        }
                    }
                }
                let new_child = if actual_target == "**" {
                    (IgnoreTreePattern::WildCardAll, None)
                } else {
                    (IgnoreTreePattern::Regular(actual_target.into()), None)
                };
                target_list.push(new_child);
            }
        }
    }

    #[allow(dead_code)]
    pub fn match_pattern<P>(&self, target: P) -> bool
    where
        P: AsRef<str>,
    {
        let components = target.as_ref().split_once("/");
        match components {
            Some((prefix, suffix)) => {
                // multi level
                self.ruleset.iter().any(|rule| {
                    (rule.1.is_none() && rule.0.match_pattern(prefix))
                        || (rule.1.is_some()
                            && match &rule.0 {
                                IgnoreTreePattern::Regular(regular) => {
                                    regular.match_pattern(prefix)
                                        && rule.1.as_ref().unwrap().match_pattern(suffix)
                                }
                                IgnoreTreePattern::WildCardAll => {
                                    let mut pending_split = Some(("", target.as_ref()));
                                    let mut matching = false;
                                    while pending_split.is_some() {
                                        let (_, pending_suffix) = pending_split.unwrap();
                                        if rule.1.as_ref().unwrap().match_pattern(pending_suffix) {
                                            matching = true;
                                            break;
                                        }
                                        pending_split = pending_suffix.split_once("/");
                                    }
                                    matching
                                }
                            })
                }) && self.exclude.iter().all(|black| {
                    (black.1.is_none() && !black.0.match_pattern(prefix))
                        || (black.1.is_some()
                            && match &black.0 {
                                IgnoreTreePattern::WildCardAll => {
                                    let mut pending_split = Some(("", target.as_ref()));
                                    let mut matching = false;
                                    while pending_split.is_some() {
                                        let (_, pending_suffix) = pending_split.unwrap();
                                        if black.1.as_ref().unwrap().match_pattern(pending_suffix) {
                                            matching = true;
                                            break;
                                        }
                                        pending_split = pending_suffix.split_once("/");
                                    }
                                    !matching
                                }
                                IgnoreTreePattern::Regular(regular) => {
                                    !regular.match_pattern(prefix)
                                        || !black.1.as_ref().unwrap().match_pattern(suffix)
                                }
                            })
                })
            }
            None => {
                // single level
                self.ruleset.iter().any(|rule| {
                    (rule.1.is_none() && rule.0.match_pattern(target.as_ref()))
                        || (rule.1.is_some()
                            && match &rule.0 {
                                IgnoreTreePattern::WildCardAll => {
                                    rule.1.as_ref().unwrap().match_pattern(target.as_ref())
                                }
                                _ => false,
                            })
                }) && self.exclude.iter().all(|black| {
                    (black.1.is_some()
                        && match &black.0 {
                            IgnoreTreePattern::WildCardAll => {
                                !black.1.as_ref().unwrap().match_pattern(target.as_ref())
                            }
                            _ => true,
                        })
                        || (black.1.is_none() && !black.0.match_pattern(target.as_ref()))
                })
            }
        }
    }
}

impl IgnoreTreeNode {
    pub fn from_path<P>(filepath: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filepath)?;
        let mut tree_node = Self::new();
        for line in BufReader::new(file).lines() {
            let content = line?;
            let content = content.trim();
            if content.is_empty()
                || (content.starts_with("#") && !content.starts_with("#ARFRIGATE:"))
            {
                continue;
            }
            let content = content.replace("#ARFRIGATE", "");
            tree_node.add_path(content.trim().trim_start_matches("/").trim_end_matches("/"));
        }
        Ok(tree_node)
    }
}

pub enum IgnoreTreeMatchHint {
    NoneMatch,
    WhiteOnly,
    BlackOnly,
    Sub,
    WildcardAllMatch,
}

impl IgnoreTreeNode {
    pub fn match_hint<P>(&self, target: P) -> IgnoreTreeMatchHint
    where
        P: AsRef<str>,
    {
        let components = target.as_ref().split_once("/");
        match components {
            Some((prefix, suffix)) => {
                // forward to the child
                let white_result = self
                    .ruleset
                    .iter()
                    .filter(|rule| rule.0.match_pattern(prefix))
                    .map(|rule| {
                        if let IgnoreTreePattern::WildCardAll = rule.0 {
                            return match &rule.1 {
                                None => IgnoreTreeMatchHint::WhiteOnly,
                                Some(_) => IgnoreTreeMatchHint::WildcardAllMatch,
                            };
                        }
                        if let Some(childrule) = &rule.1 {
                            childrule.match_hint(suffix)
                        } else {
                            IgnoreTreeMatchHint::NoneMatch
                        }
                    })
                    .reduce(|cur, result| match cur {
                        IgnoreTreeMatchHint::NoneMatch => result,
                        IgnoreTreeMatchHint::Sub => cur,
                        IgnoreTreeMatchHint::WildcardAllMatch => {
                            IgnoreTreeMatchHint::WildcardAllMatch
                        }
                        _ => match result {
                            IgnoreTreeMatchHint::NoneMatch => cur,
                            _ => result,
                        },
                    });
                let black_result = self
                    .exclude
                    .iter()
                    .filter(|rule| rule.0.match_pattern(prefix))
                    .map(|rule| {
                        if let IgnoreTreePattern::WildCardAll = rule.0 {
                            return match &rule.1 {
                                None => IgnoreTreeMatchHint::BlackOnly,
                                Some(_) => IgnoreTreeMatchHint::WildcardAllMatch,
                            };
                        }
                        if let Some(childrule) = &rule.1 {
                            match childrule.match_hint(suffix) {
                                IgnoreTreeMatchHint::WhiteOnly => IgnoreTreeMatchHint::BlackOnly,
                                IgnoreTreeMatchHint::BlackOnly => IgnoreTreeMatchHint::WhiteOnly,
                                other => other,
                            }
                        } else {
                            IgnoreTreeMatchHint::NoneMatch
                        }
                    })
                    .reduce(|cur, result| match cur {
                        IgnoreTreeMatchHint::NoneMatch => result,
                        IgnoreTreeMatchHint::Sub => cur,
                        _ => match result {
                            IgnoreTreeMatchHint::NoneMatch => cur,
                            _ => result,
                        },
                    });
                match white_result {
                    None => match black_result {
                        None => IgnoreTreeMatchHint::NoneMatch,
                        Some(black_actual) => black_actual,
                    },
                    Some(white_actual) => match black_result {
                        None => white_actual,
                        Some(black_actual) => match white_actual {
                            IgnoreTreeMatchHint::NoneMatch => black_actual,
                            IgnoreTreeMatchHint::Sub => IgnoreTreeMatchHint::Sub,
                            IgnoreTreeMatchHint::WildcardAllMatch => {
                                IgnoreTreeMatchHint::WildcardAllMatch
                            }
                            IgnoreTreeMatchHint::WhiteOnly => match black_actual {
                                IgnoreTreeMatchHint::Sub => IgnoreTreeMatchHint::Sub,
                                IgnoreTreeMatchHint::BlackOnly => IgnoreTreeMatchHint::Sub,
                                IgnoreTreeMatchHint::WildcardAllMatch => {
                                    IgnoreTreeMatchHint::WildcardAllMatch
                                }
                                _ => IgnoreTreeMatchHint::WhiteOnly,
                            },
                            _ => unreachable!("The ruleset cannot generate the BlackOnly hint"),
                        },
                    },
                }
            }
            None => {
                let white_result: Option<Option<bool>> =
                    self.ruleset.iter().fold(None, |cur, rule| {
                        if cur.is_some() && cur.unwrap().is_some_and(|inside| inside) {
                            return cur;
                        }
                        if !rule.0.match_pattern(target.as_ref()) {
                            cur
                        } else {
                            match &rule.0 {
                                IgnoreTreePattern::WildCardAll => match &rule.1 {
                                    None => Some(Some(false)),
                                    Some(_) => Some(None),
                                },
                                _ => {
                                    if rule.1.is_none() {
                                        if cur.is_none() {
                                            Some(Some(false))
                                        } else {
                                            cur
                                        }
                                    } else {
                                        Some(Some(true))
                                    }
                                }
                            }
                        }
                    });
                let black_result: Option<Option<bool>> =
                    self.exclude.iter().fold(None, |cur, rule| {
                        if cur.is_some() && cur.unwrap().is_some_and(|inside| inside) {
                            return cur;
                        }
                        if !rule.0.match_pattern(target.as_ref()) {
                            cur
                        } else {
                            match &rule.0 {
                                IgnoreTreePattern::WildCardAll => match &rule.1 {
                                    None => Some(Some(false)),
                                    Some(_) => Some(None),
                                },
                                _ => {
                                    if rule.1.is_none() {
                                        if cur.is_none() {
                                            Some(Some(false))
                                        } else {
                                            cur
                                        }
                                    } else {
                                        Some(Some(true))
                                    }
                                }
                            }
                        }
                    });

                #[allow(clippy::unnecessary_unwrap)]
                if white_result.is_none() && black_result.is_none() {
                    IgnoreTreeMatchHint::NoneMatch
                } else if white_result.is_none() {
                    let black_result = black_result.unwrap();
                    match black_result {
                        None => IgnoreTreeMatchHint::WildcardAllMatch,
                        Some(false) => IgnoreTreeMatchHint::BlackOnly,
                        Some(true) => IgnoreTreeMatchHint::Sub,
                    }
                } else if black_result.is_none() {
                    let white_result = white_result.unwrap();
                    match white_result {
                        None => IgnoreTreeMatchHint::WildcardAllMatch,
                        Some(false) => IgnoreTreeMatchHint::WhiteOnly,
                        Some(true) => IgnoreTreeMatchHint::Sub,
                    }
                } else if white_result.unwrap().is_none() || black_result.unwrap().is_none() {
                    IgnoreTreeMatchHint::WildcardAllMatch
                } else {
                    IgnoreTreeMatchHint::Sub
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let pattern: IgnoreMatchPattern = String::from("*abc*df*").into();
        assert!(pattern.match_pattern("2342abcaasdfffss"));
        assert!(pattern.match_pattern("abcdf"));
        assert!(pattern.match_pattern("ddaaeeabcdf3d2"));
        assert!(pattern.match_pattern("abc33e3ddf"));
        assert!(!pattern.match_pattern("abcd"));
    }

    #[test]
    fn test_special_matching() {
        let pattern: IgnoreMatchPattern = String::from("*").into();
        assert!(pattern.match_pattern(""));
        assert!(pattern.match_pattern("*"));
        assert!(pattern.match_pattern("abcdf"));
        let pattern: IgnoreMatchPattern = String::from("").into();
        assert!(pattern.match_pattern(""));
        assert!(!pattern.match_pattern("*"));
    }

    #[test]
    fn test_corner_matching() {
        let pattern: IgnoreMatchPattern = "abc".into();
        assert!(pattern.match_pattern("abc"));
        assert!(!pattern.match_pattern("abcd"));
        let pattern: IgnoreMatchPattern = "abcdefg".into();
        assert!(pattern.match_pattern("abcdefg"));
        assert!(!pattern.match_pattern("abcd"));
        let pattern: IgnoreMatchPattern = "a*c".into();
        assert!(pattern.match_pattern("abc"));
    }

    #[test]
    fn test_basic_tree() {
        let mut tree_pattern: IgnoreTreeNode = "test/**/*.jpg".into();
        assert!(!tree_pattern.match_pattern("test"));
        assert!(tree_pattern.match_pattern("test/1.jpg"));
        assert!(tree_pattern.match_pattern("test/abc/32.jpg"));
        assert!(!tree_pattern.match_pattern("testds"));
        tree_pattern.add_path("!test/**/*.png");
        assert!(!tree_pattern.match_pattern("test/abc.png"));
        assert!(!tree_pattern.match_pattern("test/fsds/abfds.png"));
        tree_pattern.add_path("!test/test*");
        assert!(tree_pattern.match_pattern("test/tewews/fsdfsdd3.jpg"));
        assert!(!tree_pattern.match_pattern("test/testa"));
        assert!(tree_pattern.match_pattern("test/terwe/fdsdds.jpg"));
    }

    #[test]
    fn test_match_hint() {
        let mut tree_pattern = IgnoreTreeNode::default();
        tree_pattern.add_path("test");
        tree_pattern.add_path("test/fded");
        tree_pattern.add_path("test/fded/**");
        tree_pattern.add_path("test/fdgr");
        tree_pattern.add_path("test/black");
        tree_pattern.add_path("!test/black/fdssds");
        assert!(matches!(
            tree_pattern.match_hint("test/er34r"),
            IgnoreTreeMatchHint::NoneMatch
        ));
        assert!(matches!(
            tree_pattern.match_hint("test"),
            IgnoreTreeMatchHint::Sub
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/fded"),
            IgnoreTreeMatchHint::Sub
        ));
        assert!(matches!(
            tree_pattern.match_hint("test"),
            IgnoreTreeMatchHint::Sub
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/fdgr"),
            IgnoreTreeMatchHint::WhiteOnly
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/black"),
            IgnoreTreeMatchHint::Sub
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/black/fdfdfd"),
            IgnoreTreeMatchHint::NoneMatch
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/fded/fdfdd"),
            IgnoreTreeMatchHint::WhiteOnly
        ));
        assert!(matches!(
            tree_pattern.match_hint("test/black/fdssds"),
            IgnoreTreeMatchHint::BlackOnly
        ));
    }
}

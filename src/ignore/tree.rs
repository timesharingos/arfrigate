use std::convert::AsRef;

pub enum IgnoreMatchPatternType {
    Wildcard,
    Strict(String),
}

pub struct IgnoreMatchPattern {
    original_pattern: String,
    pattern_part: Vec<IgnoreMatchPatternType>,
}

impl From<String> for IgnoreMatchPattern {
    fn from(value: String) -> Self {
        let str_value: &str = &value;
        str_value.into()
    }
}
impl From<&str> for IgnoreMatchPattern {
    fn from(value: &str) -> Self {
        if value.is_empty() {
            return Self {
                original_pattern: value.to_string(),
                pattern_part: vec![],
            };
        }
        if value == "*" {
            return Self {
                original_pattern: value.to_string(),
                pattern_part: vec![IgnoreMatchPatternType::Wildcard],
            };
        }
        let mut pattern_part: Vec<IgnoreMatchPatternType> = value
            .split("*")
            .flat_map(|x| {
                vec![
                    IgnoreMatchPatternType::Strict(x.to_string()),
                    IgnoreMatchPatternType::Wildcard,
                ]
            })
            .filter(|x| match x {
                IgnoreMatchPatternType::Strict(part) => !part.is_empty(),
                IgnoreMatchPatternType::Wildcard => true,
            })
            .collect();
        pattern_part.remove(pattern_part.len() - 1);
        Self {
            pattern_part,
            original_pattern: value.to_string(),
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
    pub fn get_pattern(&self) -> &Vec<IgnoreMatchPatternType> {
        &self.pattern_part
    }
    pub fn match_pattern<P>(&self, target: P) -> bool
    where
        P: AsRef<str>,
    {
        if self.pattern_part.is_empty() {
            return target.as_ref().is_empty();
        }
        let mut match_index = 0_usize;
        let mut wildcard_mode = false;
        for pattern_type in &self.pattern_part {
            match pattern_type {
                IgnoreMatchPatternType::Wildcard => {
                    wildcard_mode = true;
                }
                IgnoreMatchPatternType::Strict(part) => {
                    if wildcard_mode {
                        //try to find the next literal value
                        let pending_index = &target.as_ref()[match_index..].find(part);
                        if pending_index.is_none() {
                            return false;
                        }
                        match_index = match_index + pending_index.unwrap() + part.len();
                        wildcard_mode = false;
                    } else {
                        //compare the literal value
                        let pattern_len = part.len();
                        if match_index + pattern_len > target.as_ref().len() {
                            return false;
                        }
                        let target_part =
                            &target.as_ref()[match_index..(match_index + pattern_len)];
                        if target_part != part {
                            return false;
                        } else {
                            match_index += pattern_len;
                        }
                    }
                }
            }
        }
        if wildcard_mode {
            true
        } else {
            match_index == target.as_ref().len()
        }
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
        //TODO
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

    pub fn match_pattern<P>(&self, target: P) -> bool
    where
        P: AsRef<str>,
    {
        let components = target.as_ref().split_once("/");
        match components {
            Some((prefix, suffix)) => {
                // multi level
                self.ruleset.iter().any(|rule| {
                    (rule.1.is_none() && matches!(&rule.0, IgnoreTreePattern::WildCardAll))
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
                    (black.1.is_none() && !matches!(&black.0, IgnoreTreePattern::WildCardAll))
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
                        || rule.1.is_some()
                            && match &rule.0 {
                                IgnoreTreePattern::WildCardAll => {
                                    rule.1.as_ref().unwrap().match_pattern(target.as_ref())
                                }
                                _ => false,
                            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let pattern: IgnoreMatchPattern = String::from("*abc*df*").into();
        assert_eq!(pattern.get_pattern().len(), 5);
        assert!(pattern.match_pattern("2342abcaasdfffss"));
        assert!(pattern.match_pattern("abcdf"));
        assert!(pattern.match_pattern("ddaaeeabcdf3d2"));
        assert!(pattern.match_pattern("abc33e3ddf"));
        assert!(!pattern.match_pattern("abcd"));
    }

    #[test]
    fn test_special_matching() {
        let pattern: IgnoreMatchPattern = String::from("*").into();
        assert_eq!(pattern.get_pattern().len(), 1);
        assert!(pattern.match_pattern(""));
        assert!(pattern.match_pattern("*"));
        assert!(pattern.match_pattern("abcdf"));
        let pattern: IgnoreMatchPattern = String::from("").into();
        assert_eq!(pattern.get_pattern().len(), 0);
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
}

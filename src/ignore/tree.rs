use std::convert::AsRef;

pub enum IgnoreMatchPatternType {
    Wildcard,
    Strict(String),
}

pub struct IgnoreMatchPattern {
    pattern_part: Vec<IgnoreMatchPatternType>,
}

impl From<String> for IgnoreMatchPattern {
    fn from(value: String) -> Self {
        if value.is_empty() {
            return Self {
                pattern_part: vec![],
            };
        }
        if value == "*" {
            return Self {
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
        Self { pattern_part }
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
                        match_index = pending_index.unwrap() + part.len();
                    } else {
                        //compare the literal value
                        let pattern_len = part.len();
                        let target_part =
                            &target.as_ref()[match_index..(match_index + pattern_len)];
                        if target_part != part {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

pub struct IgnoreTreeNode {}

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
}

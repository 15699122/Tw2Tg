//! Deterministic local TagEngine primitives.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagInput<'a> {
    pub username: Option<&'a str>,
    pub text: &'a str,
    pub tweet_type: &'a str,
    pub media_types: &'a [&'a str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagRule {
    Username { value: String, tag: String },
    TextContains { value: String, tag: String },
    TweetType { value: String, tag: String },
    MediaType { value: String, tag: String },
}

impl TagRule {
    pub fn matches(&self, input: &TagInput<'_>) -> bool {
        match self {
            Self::Username { value, .. } => input
                .username
                .is_some_and(|username| username.eq_ignore_ascii_case(value)),
            Self::TextContains { value, .. } => {
                input.text.to_lowercase().contains(&value.to_lowercase())
            }
            Self::TweetType { value, .. } => input.tweet_type.eq_ignore_ascii_case(value),
            Self::MediaType { value, .. } => input
                .media_types
                .iter()
                .any(|media| media.eq_ignore_ascii_case(value)),
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            Self::Username { tag, .. }
            | Self::TextContains { tag, .. }
            | Self::TweetType { tag, .. }
            | Self::MediaType { tag, .. } => tag,
        }
    }
}

pub fn evaluate_tags<'a>(rules: &'a [TagRule], input: &TagInput<'_>) -> Vec<&'a str> {
    rules
        .iter()
        .filter(|rule| rule.matches(input))
        .map(TagRule::tag)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_username_text_type_and_media_rules() {
        let rules = vec![
            TagRule::Username {
                value: "Alice".into(),
                tag: "person".into(),
            },
            TagRule::TextContains {
                value: "launch".into(),
                tag: "announcement".into(),
            },
            TagRule::TweetType {
                value: "quote".into(),
                tag: "context".into(),
            },
            TagRule::MediaType {
                value: "video".into(),
                tag: "watch".into(),
            },
        ];
        let input = TagInput {
            username: Some("alice"),
            text: "Product launch",
            tweet_type: "quote",
            media_types: &["video"],
        };
        assert_eq!(
            evaluate_tags(&rules, &input),
            vec!["person", "announcement", "context", "watch"]
        );
    }
}

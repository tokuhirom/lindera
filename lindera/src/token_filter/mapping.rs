use std::{borrow::Cow, collections::HashMap};

use lindera_core::token_filter::TokenFilter;
use serde::{Deserialize, Serialize};
use yada::{builder::DoubleArrayBuilder, DoubleArray};

use crate::{error::LinderaErrorKind, LinderaResult, Token};

pub const MAPPING_TOKEN_FILTER_NAME: &str = "mapping";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MappingTokenFilterConfig {
    pub mapping: HashMap<String, String>,
}

impl MappingTokenFilterConfig {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self { mapping: map }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone)]
pub struct MappingTokenFilter {
    config: MappingTokenFilterConfig,
    trie: DoubleArray<Vec<u8>>,
}

impl MappingTokenFilter {
    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        let config = MappingTokenFilterConfig::from_slice(data)?;

        Self::new(config)
    }

    pub fn new(config: MappingTokenFilterConfig) -> LinderaResult<Self> {
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = config.mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let data = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        let trie = DoubleArray::new(data);

        Ok(Self { config, trie })
    }
}

impl TokenFilter for MappingTokenFilter {
    fn name(&self) -> &'static str {
        MAPPING_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let text = token.text.to_string();

            let mut result = String::new();
            let mut start = 0_usize;
            let len = text.len();

            while start < len {
                let suffix = &text[start..];
                match self
                    .trie
                    .common_prefix_search(suffix.as_bytes())
                    .last()
                    .map(|(_offset_len, prefix_len)| prefix_len)
                {
                    Some(prefix_len) => {
                        let surface = &text[start..start + prefix_len];
                        let replacement = &self.config.mapping[surface];

                        result.push_str(replacement);

                        // move start offset
                        start += prefix_len;
                    }
                    None => {
                        match suffix.chars().next() {
                            Some(c) => {
                                result.push(c);

                                // move start offset
                                start += c.len_utf8();
                            }
                            None => break,
                        }
                    }
                }
            }

            token.text = Cow::Owned(result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use lindera_core::token_filter::TokenFilter;

    use crate::{
        token_filter::mapping::{MappingTokenFilter, MappingTokenFilterConfig},
        Token,
    };

    #[test]
    fn test_mapping_token_filter_config_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = MappingTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();
        assert_eq!("ア", config.mapping.get("ｱ").unwrap());
    }

    #[test]
    fn test_mapping_token_filter_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result = MappingTokenFilter::from_slice(config_str.as_bytes());
        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_mapping_token_filter_apply() {
        {
            let config_str = r#"
            {
                "mapping": {
                    "籠": "篭"
                }
            }
            "#;
            let filter = MappingTokenFilter::from_slice(config_str.as_bytes()).unwrap();

            let mut tokens: Vec<Token> = vec![
                Token {
                    text: Cow::Borrowed("籠原"),
                    details: None,
                    byte_start: 0,
                    byte_end: 6,
                },
                Token {
                    text: Cow::Borrowed("駅"),
                    details: None,
                    byte_start: 6,
                    byte_end: 9,
                },
            ];

            filter.apply(&mut tokens).unwrap();

            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].text, "篭原");
            assert_eq!(tokens[1].text, "駅");
        }
    }
}
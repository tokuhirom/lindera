use std::path::{Path, PathBuf};

use lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder;
use lindera_core::{
    dictionary::Dictionary, dictionary_builder::DictionaryBuilder, file_util::read_file,
    user_dictionary::UserDictionary,
};
use lindera_dictionary::{load_dictionary_from_kind, DictionaryKind};
use lindera_ipadic_builder::ipadic_builder::IpadicBuilder;
use lindera_ko_dic_builder::ko_dic_builder::KoDicBuilder;
use lindera_unidic_builder::unidic_builder::UnidicBuilder;
use serde::{Deserialize, Serialize};

use crate::{error::LinderaErrorKind, LinderaResult};

/// Dictionary config
///
/// Use this if you want to use a dictionary when tokenizing.
///
/// Either `kind` or `path` must be specified.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictionaryConfig {
    /// Specify the kind of dictionary (IPADIC, UniDic, ko-dic, CC-CEDICT) if a self-contained dictionary is used for tokenization.
    pub kind: Option<DictionaryKind>,
    /// Specifies the path to a pre-built external dictionary if one is used.
    pub path: Option<PathBuf>,
}

/// User dictionary config
///
/// Use this if you want to use a user dictionary when tokenizing.
///
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserDictionaryConfig {
    /// Path to the user dictionary file.
    pub path: PathBuf,
    /// If the user dictionary was in CSV format, specify the dictionary type (IPADIC, UniDic, ko-dic or CC-CEDICT).
    pub kind: Option<DictionaryKind>,
}

pub fn resolve_builder(
    dictionary_type: DictionaryKind,
) -> LinderaResult<Box<dyn DictionaryBuilder>> {
    match dictionary_type {
        DictionaryKind::IPADIC => Ok(Box::new(IpadicBuilder::new())),
        DictionaryKind::UniDic => Ok(Box::new(UnidicBuilder::new())),
        DictionaryKind::KoDic => Ok(Box::new(KoDicBuilder::new())),
        DictionaryKind::CcCedict => Ok(Box::new(CcCedictBuilder::new())),
    }
}

pub fn build_dictionary(
    dictionary_type: DictionaryKind,
    input_dir: &Path,
    output_dir: &Path,
) -> LinderaResult<()> {
    resolve_builder(dictionary_type)?.build_dictionary(input_dir, output_dir)
}

pub fn build_user_dictionary(
    dictionary_type: DictionaryKind,
    input_file: &Path,
    output_dir: &Path,
) -> LinderaResult<()> {
    let output_file = if let Some(filename) = input_file.file_name() {
        let mut output_file = Path::new(output_dir).join(filename);
        output_file.set_extension("bin");
        output_file
    } else {
        return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!("failed to get filename")));
    };

    resolve_builder(dictionary_type)?.build_user_dictionary(input_file, &output_file)
}

pub fn load_dictionary(dictionary_config: DictionaryConfig) -> LinderaResult<Dictionary> {
    match dictionary_config.kind {
        Some(kind) => {
            // The dictionary specified by the feature flag will be loaded.
            load_dictionary_from_kind(kind)
        }
        None => {
            match dictionary_config.path {
                Some(path) => {
                    // load external dictionary from path
                    lindera_dictionary::load_dictionary(path)
                }
                None => Err(LinderaErrorKind::Args
                    .with_error(anyhow::anyhow!("Dictionary must be specified"))),
            }
        }
    }
}

pub fn build_user_dictionary_from_csv(
    kind: DictionaryKind,
    path: PathBuf,
) -> LinderaResult<UserDictionary> {
    resolve_builder(kind)?
        .build_user_dict(&path)
        .map_err(|err| LinderaErrorKind::DictionaryBuildError.with_error(err))
}

pub fn load_user_dictionary_from_bin(data: &[u8]) -> LinderaResult<UserDictionary> {
    UserDictionary::load(data)
}

pub fn load_user_dictionary(
    dictionary_config: UserDictionaryConfig,
) -> LinderaResult<UserDictionary> {
    match dictionary_config.path.extension() {
        Some(ext) => match ext.to_str() {
            Some("csv") => match dictionary_config.kind {
                Some(kind) => build_user_dictionary_from_csv(kind, dictionary_config.path),
                None => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "Dictionary type must be specified if CSV file specified"
                ))),
            },
            Some("bin") => load_user_dictionary_from_bin(&read_file(&dictionary_config.path)?),
            _ => Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                "Invalid user dictionary source file extension"
            ))),
        },
        None => Err(LinderaErrorKind::Args
            .with_error(anyhow::anyhow!("Invalid user dictionary source file"))),
    }
}
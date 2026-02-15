use std::path::PathBuf;
use std::sync::Arc;

use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use tantivy::tokenizer::TextAnalyzer;
use tantivy_tokenizer_api::{Token, TokenStream};

use sudachi_tantivy::SudachiTokenizer;

#[test]
fn test_tokenize() {
    let tokens = token_stream_helper("選挙管理委員会");

    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].text, "選挙");
    assert_eq!(tokens[1].text, "管理");
    assert_eq!(tokens[2].text, "委員");
    assert_eq!(tokens[3].text, "会");
}

#[test]
fn test_mix_jp_alphabet() {
    let tokens = token_stream_helper("sudachi.rs は日本語形態素解析器 Sudachi の Rust 実装です。");

    assert_eq!(tokens.len(), 16);
    assert_eq!(tokens[0].text, "sudachi");
    assert_eq!(tokens[1].text, ".");
    assert_eq!(tokens[2].text, "rs");
    assert_eq!(tokens[3].text, "は");
    assert_eq!(tokens[4].text, "日本");
    assert_eq!(tokens[5].text, "語");
    assert_eq!(tokens[6].text, "形態");
    assert_eq!(tokens[7].text, "素");
    assert_eq!(tokens[8].text, "解析");
    assert_eq!(tokens[9].text, "器");
    assert_eq!(tokens[10].text, "Sudachi");
    assert_eq!(tokens[11].text, "の");
    assert_eq!(tokens[12].text, "Rust");
    assert_eq!(tokens[13].text, "実装");
    assert_eq!(tokens[14].text, "です");
    assert_eq!(tokens[15].text, "。");
}

fn token_stream_helper(text: &str) -> Vec<Token> {
    let mut analyzer = analyzer();

    let mut token_stream = analyzer.token_stream(text);
    let mut tokens: Vec<Token> = vec![];
    let mut add_token = |token: &Token| {
        tokens.push(token.clone());
    };
    token_stream.process(&mut add_token);
    tokens
}

fn analyzer() -> TextAnalyzer {
    let dict_path = std::env::var("SUDACHI_DICT_PATH")
        .map(|p| PathBuf::from(p))
        .expect("Environemt variable SUDACHI_DICT_PATH is not defined");
    let config = Config::new(None, None, Some(dict_path)).expect("Failed to load config file");

    let jp_dict = JapaneseDictionary::from_cfg(&config)
        .unwrap_or_else(|e| panic!("Failed to create dictionary: {:?}", e));

    let dict = Arc::new(jp_dict);
    let tokenizer = SudachiTokenizer::new(dict);
    TextAnalyzer::from(tokenizer)
}

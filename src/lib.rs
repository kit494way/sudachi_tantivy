//! Sudachi tokenizer for Tantivy.

use std::str;

use sudachi::analysis::Mode;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::prelude::MorphemeList;
use tantivy_tokenizer_api::{Token, TokenStream, Tokenizer};

/// Tokenize the text using Sudachi.
pub struct SudachiTokenizer<D: DictionaryAccess> {
    token: Token,
    stateful_tokenizer: StatefulTokenizer<D>,
    debug: bool,
}

/// TokenStream produced by SudachiTokenizer.
pub struct SudachiTokenStream<'a, D: DictionaryAccess> {
    token: &'a mut Token,
    morphemes: MorphemeList<D>,
    index: usize,
}

impl<D: DictionaryAccess> SudachiTokenizer<D> {
    /// Creates a new SudachiTokenizer.
    pub fn new(dict: D) -> Self {
        Self {
            token: Token::default(),
            stateful_tokenizer: StatefulTokenizer::new(dict, Mode::A),
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) -> &Self {
        self.debug = debug;
        self.stateful_tokenizer.set_debug(debug);
        self
    }
}

impl<D: DictionaryAccess + Clone> Clone for SudachiTokenizer<D> {
    fn clone(&self) -> Self {
        let mut stateful_tokenizer = StatefulTokenizer::new(
            self.stateful_tokenizer.dict_clone(),
            self.stateful_tokenizer.mode(),
        );
        stateful_tokenizer.set_debug(self.debug);
        Self {
            token: Token::default(),
            stateful_tokenizer,
            debug: self.debug,
        }
    }
}

impl<D: DictionaryAccess + 'static + Send + Sync + Clone> Tokenizer for SudachiTokenizer<D> {
    type TokenStream<'a> = SudachiTokenStream<'a, D>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.token.reset();

        self.stateful_tokenizer.reset().push_str(text);

        let mut morphemes = MorphemeList::empty(self.stateful_tokenizer.dict_clone());
        match self.stateful_tokenizer.do_tokenize() {
            Ok(_) => morphemes
                .collect_results(&mut self.stateful_tokenizer)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to collect tokens, text: {}, error: {}", text, e)
                }),
            Err(e) => eprintln!("Tokenization failed, text: {}, error: {}", text, e),
        };

        SudachiTokenStream::new(&mut self.token, morphemes)
    }
}

impl<'a, D: DictionaryAccess> SudachiTokenStream<'a, D> {
    /// Creates a new `SudachiTokenStream.`
    pub fn new(token: &'a mut Token, morphemes: MorphemeList<D>) -> Self {
        Self {
            token,
            morphemes,
            index: 0,
        }
    }
}

impl<'a, D: DictionaryAccess> TokenStream for SudachiTokenStream<'a, D> {
    fn advance(&mut self) -> bool {
        while self.index < self.morphemes.len() {
            let m = self.morphemes.get(self.index);
            self.index += 1;
            if let Some(pos) = m.part_of_speech().get(0)
                && pos == "空白"
            {
                continue;
            }

            self.token.position = self.token.position.wrapping_add(1);
            self.token.offset_from = m.begin();
            self.token.offset_to = m.end() + 1;
            self.token.text.clear();
            self.token.text.push_str(m.surface().as_ref());

            return true;
        }

        false
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

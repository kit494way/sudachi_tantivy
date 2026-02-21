//! Sudachi tokenizer for Tantivy.

use std::str;

use log::error;
use sudachi::analysis::Mode;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::prelude::MorphemeList;
use tantivy_tokenizer_api::{Token, TokenStream, Tokenizer};

// Maximum size of text to be tokenized, in bytes.
// The text size that can be tokenized is limited in Sudachi.
// See https://github.com/WorksApplications/sudachi.rs/blob/5e8cc9250a712aa328a43381f7a9eb7f245bdd4a/sudachi/src/input_text/buffer/mod.rs#L32
const SUDACHI_MAX_LENGTH: usize = u16::MAX as usize / 4 * 3;

/// Tokenize the text using Sudachi.
pub struct SudachiTokenizer<D: DictionaryAccess> {
    token: Token,
    stateful_tokenizer: StatefulTokenizer<D>,
    debug: bool,
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

    pub fn set_mode(&mut self, mode: Mode) -> &Self {
        self.stateful_tokenizer.set_mode(mode);
        self
    }
}

/// TokenStream produced by SudachiTokenizer.
pub struct SudachiTokenStream<'a, D: DictionaryAccess> {
    chunks: TextChunkIterator<'a>,
    chunk_size: usize,
    tokenizer: &'a mut SudachiTokenizer<D>,
    morphemes: MorphemeList<D>,
    index: usize,
    offset: usize,
    token_position: usize,
}

impl<'a, D: DictionaryAccess + Clone> SudachiTokenStream<'a, D> {
    fn new(tokenizer: &'a mut SudachiTokenizer<D>, text: &'a str) -> Self {
        let chunks = TextChunkIterator::new(text, SUDACHI_MAX_LENGTH);
        let morphemes = MorphemeList::empty(tokenizer.stateful_tokenizer.dict_clone());

        Self {
            chunks,
            chunk_size: 0,
            tokenizer,
            morphemes,
            index: 0,
            offset: 0,
            token_position: 0,
        }
    }
}

impl<'a, D: DictionaryAccess> SudachiTokenStream<'a, D> {
    fn has_next(&mut self) -> bool {
        if self.index < self.morphemes.len() {
            return true;
        }

        match self.chunks.next() {
            Some(chunk) => {
                self.index = 0;
                self.offset += self.chunk_size;
                self.chunk_size = chunk.len();
                self.morphemes.clear();
                self.tokenizer.tokenize(chunk, &mut self.morphemes);
                !self.morphemes.is_empty()
            }
            None => false,
        }
    }
}

impl<'a, D: DictionaryAccess> TokenStream for SudachiTokenStream<'a, D> {
    fn advance(&mut self) -> bool {
        while self.has_next() {
            let m = self.morphemes.get(self.index);
            self.index += 1;
            if let Some(pos) = m.part_of_speech().first()
                && pos == "空白"
            {
                continue;
            }

            self.token_position = self.token_position.wrapping_add(1);
            self.tokenizer.token.position = self.token_position;
            self.tokenizer.token.offset_from = self.offset + m.begin();
            self.tokenizer.token.offset_to = self.offset + m.end() + 1;
            self.tokenizer.token.text.clear();
            self.tokenizer.token.text.push_str(m.surface().as_ref());

            return true;
        }

        false
    }

    fn token(&self) -> &Token {
        &self.tokenizer.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokenizer.token
    }
}

impl<D: DictionaryAccess> SudachiTokenizer<D> {
    fn tokenize<'a>(&'a mut self, text: &'a str, out: &mut MorphemeList<D>) {
        self.token.reset();

        self.stateful_tokenizer.reset().push_str(text);
        match self.stateful_tokenizer.do_tokenize() {
            Ok(_) => out
                .collect_results(&mut self.stateful_tokenizer)
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to collect tokens, text: {}, error: {}",
                        truncate_chars(text, 100),
                        e
                    )
                }),
            Err(e) => error!(
                "Tokenization failed, text: {}, error: {}",
                truncate_chars(text, 100),
                e
            ),
        };
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
        SudachiTokenStream::new(self, text)
    }
}

#[derive(Debug)]
struct TextChunkIterator<'a> {
    text: &'a str,
    max_chunk_size: usize,
    start: usize,
}

impl<'a> TextChunkIterator<'a> {
    fn new(text: &'a str, max_chunk_size: usize) -> Self {
        Self {
            text,
            max_chunk_size,
            start: 0,
        }
    }
}

impl<'a> Iterator for TextChunkIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.text.len() {
            let limit = self
                .text
                .floor_char_boundary(self.start + self.max_chunk_size);
            if limit >= self.text.len() {
                let chunk = &self.text[self.start..];
                self.start = self.text.len();
                return Some(chunk);
            }

            let part = &self.text[self.start..limit];
            let end = rfind_end_of_sentence(part);
            let chunk = &self.text[self.start..(self.start + end)];
            self.start += end;
            Some(chunk)
        } else {
            None
        }
    }
}

fn rfind_end_of_sentence(txt: &str) -> usize {
    let mut text = txt;
    while let Some(i) = text.rfind("\n") {
        let s = &text[..i + 1];
        if s.ends_with(".\n")
            || s.ends_with(".\r\n")
            || s.ends_with("。\n")
            || s.ends_with("。\r\n")
            || s.ends_with("\n\n")
            || s.ends_with("\r\n\r\n")
        {
            return i + 1;
        }

        text = &text[..i];
    }

    text.len()
}

fn truncate_chars(s: &str, len: usize) -> String {
    if s.len() > len {
        let mut ss = s.chars().take(len).collect::<String>();
        ss.push_str("...");
        ss
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rfind_eos_jp_period() {
        let sentence = "日本語のトークナイザーです。\n";
        let subject = format!("{}Sudachi を使います", sentence);
        let end = rfind_end_of_sentence(subject.as_str());

        assert_eq!(sentence.len(), end);
    }

    #[test]
    fn rfind_eos_en_period() {
        let sentence = "This is a Japanese tokenizer.\n";
        let subject = format!("{}This uses Sudachi", sentence);
        let end = rfind_end_of_sentence(subject.as_str());

        assert_eq!(sentence.len(), end);
    }

    #[test]
    fn rfind_eos_double_lf() {
        let sentence = "日本語の Tokenizer です。\n\n";
        let subject = format!("{}Sudachi を使います", sentence);
        let end = rfind_end_of_sentence(subject.as_str());

        assert_eq!(sentence.len(), end);
    }

    #[test]
    fn text_chunk() {
        let text = "This is a test.\nThis is a second line.\nThis is a third line.";
        let max_chunk_size = 25;
        let mut it = TextChunkIterator::new(text, max_chunk_size);

        assert_eq!(Some("This is a test.\n"), it.next());
        assert_eq!(Some("This is a second line.\n"), it.next());
        assert_eq!(Some("This is a third line."), it.next());
        assert_eq!(None, it.next());
    }
}

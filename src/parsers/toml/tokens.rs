use self::Token::*;
use std::borrow::Cow;
use std::string::String as StdString;
use std::{str, string};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for (usize, usize) {
    fn from(Span { start, end }: Span) -> (usize, usize) {
        (start, end)
    }
}

#[derive(Debug, Clone)]
enum MaybeString {
    NotEscaped(usize),
    Owned(string::String),
}

#[derive(Eq, PartialEq, Debug)]
pub enum Token<'a> {
    WhiteSpace(&'a str),
    NewLine,
    Comment(&'a str),

    Equals,
    Period,
    Comma,
    Colon,
    Plus,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,

    Keylike(&'a str),
    String {
        src: &'a str,
        value: Cow<'a, str>,
        multiline: bool,
    },
}

#[derive(Eq, PartialEq, Debug)]
pub enum Error {
    InvalidCharInString(usize, char),
    InvalidEscape(usize, char),
    InvalidHexEscape(usize, char),
    InvalidEscapeValue(usize, u32),
    NewlineInString(usize),
    Unexpected(usize, char),
    UnterminatedString(usize),
    NewlineInTableKey(usize),
    MultilineStringKey(usize),
    Wanted {
        at: usize,
        expected: &'static str,
        found: &'static str,
    },
}

#[derive(Clone)]
pub struct Tokenizer<'a> {
    src: &'a str,
    chars: str::CharIndices<'a>,
}

fn is_keylike(ch: char) -> bool {
    ('A' <= ch && ch <= 'Z')
        || ('a' <= ch && ch <= 'z')
        || ('0' <= ch && ch <= '9')
        || ch == '-'
        || ch == '_'
}

impl<'a> Tokenizer<'a> {
    pub fn new(src: &'a str) -> Tokenizer<'a> {
        let mut t = Tokenizer {
            src,
            chars: src.char_indices(),
        };

        t.eat('\u{feff}');
        t
    }

    pub fn tokenize(&mut self) -> Result<Option<(Span, Token<'a>)>, Error> {
        let (start, token) = match self.get_next_char() {
            Some((start, ' ')) => (start, self.whitespace(start)),
            Some((start, '\t')) => (start, self.whitespace(start)),
            Some((start, '\n')) => (start, NewLine),
            Some((start, '#')) => (start, self.comment(start)),
            Some((start, '=')) => (start, Equals),
            Some((start, '.')) => (start, Period),
            Some((start, ',')) => (start, Comma),
            Some((start, ':')) => (start, Colon),
            Some((start, '+')) => (start, Plus),
            Some((start, '{')) => (start, LeftBrace),
            Some((start, '}')) => (start, RightBrace),
            Some((start, '[')) => (start, LeftBracket),
            Some((start, ']')) => (start, RightBracket),
            Some((start, '\'')) => {
                return self
                    .literal_string(start)
                    .map(|t| Some((self.calculate_span(start), t)))
            }
            Some((start, '"')) => {
                return self
                    .basic_string(start)
                    .map(|t| Some((self.calculate_span(start), t)))
            }
            Some((start, ch)) if is_keylike(ch) => (start, self.keylike(start)),
            Some((start, ch)) => return Err(Error::Unexpected(start, ch)),
            None => return Ok(None),
        };

        let span = self.calculate_span(start);
        Ok(Some((span, token)))
    }

    fn get_next_char(&mut self) -> Option<(usize, char)> {
        self.chars.next()
    }

    fn eat(&mut self, target_char: char) -> bool {
        match self.chars.clone().next() {
            Some((_, next_char)) if target_char == next_char => {
                self.get_next_char();
                true
            }
            _ => false,
        }
    }

    fn current(&mut self) -> usize {
        self.chars
            .clone()
            .next()
            .map(|i| i.0)
            .unwrap_or_else(|| self.src.len())
    }

    fn keylike(&mut self, start: usize) -> Token<'a> {
        while let Some((_, ch)) = self.peek_next() {
            if !is_keylike(ch) {
                break;
            }
            self.get_next_char();
        }
        Keylike(&self.src[start..self.current()])
    }

    fn read_string(
        &mut self,
        delim: char,
        start: usize,
        new_ch: &mut dyn FnMut(
            &mut Tokenizer<'_>,
            &mut MaybeString,
            bool,
            usize,
            char,
        ) -> Result<(), Error>,
    ) -> Result<Token<'a>, Error> {
        let mut multiline = false;
        if self.eat(delim) {
            if self.eat(delim) {
                multiline = true;
            } else {
                return Ok(String {
                    src: &self.src[start..self.current()],
                    value: Cow::Borrowed(""),
                    multiline,
                });
            }
        }

        let mut val = MaybeString::NotEscaped(self.current());
        let mut n = 0;

        'outer: loop {
            n += 1;
            match self.get_next_char() {
                Some((i, '\n')) => {
                    if multiline {
                        if self.src.as_bytes()[i] == b'\r' {
                            val.owned(&self.src[..i]);
                        }
                        if n == 1 {
                            val = MaybeString::NotEscaped(self.current());
                        } else {
                            val.push('\n');
                        }
                        continue;
                    } else {
                        return Err(Error::NewlineInString(i));
                    }
                }
                Some((mut i, ch)) if ch == delim => {
                    if multiline {
                        if !self.eat(delim) {
                            val.push(delim);
                            continue 'outer;
                        }
                        if !self.eat(delim) {
                            val.push(delim);
                            val.push(delim);
                            continue 'outer;
                        }
                        if self.eat(delim) {
                            val.push(delim);
                            i += 1;
                        }
                        if self.eat(delim) {
                            val.push(delim);
                            i += 1;
                        }
                    }
                    return Ok(String {
                        src: &self.src[start..self.current()],
                        value: val.into_cow(&self.src[..i]),
                        multiline,
                    });
                }
                Some((i, c)) => new_ch(self, &mut val, multiline, i, c)?,
                None => return Err(Error::UnterminatedString(start)),
            }
        }
    }

    fn basic_string(&mut self, start: usize) -> Result<Token<'a>, Error> {
        self.read_string('"', start, &mut |_tokenizer, val, multi, i, ch| match ch {
            '\\' => {
                val.owned(&_tokenizer.src[..i]);
                match _tokenizer.chars.next() {
                    Some((_, '"')) => val.push('"'),
                    Some((_, '\\')) => val.push('\\'),
                    Some((_, 'b')) => val.push('\u{8}'),
                    Some((_, 'f')) => val.push('\u{c}'),
                    Some((_, 'n')) => val.push('\n'),
                    Some((_, 'r')) => val.push('\r'),
                    Some((_, 't')) => val.push('\t'),
                    Some((i, c @ 'u')) | Some((i, c @ 'U')) => {
                        let len = if c == 'u' { 4 } else { 8 };
                        val.push(_tokenizer.hex(start, i, len)?);
                    }
                    Some((i, c @ ' ')) | Some((i, c @ '\t')) | Some((i, c @ '\n')) if multi => {
                        if c != '\n' {
                            while let Some((_, ch)) = _tokenizer.chars.clone().next() {
                                match ch {
                                    ' ' | '\t' => {
                                        _tokenizer.chars.next();
                                        continue;
                                    }
                                    '\n' => {
                                        _tokenizer.chars.next();
                                        break;
                                    }
                                    _ => return Err(Error::InvalidEscape(i, c)),
                                }
                            }
                        }
                        while let Some((_, ch)) = _tokenizer.chars.clone().next() {
                            match ch {
                                ' ' | '\t' | '\n' => {
                                    _tokenizer.chars.next();
                                }
                                _ => break,
                            }
                        }
                    }
                    Some((i, c)) => return Err(Error::InvalidEscape(i, c)),
                    None => return Err(Error::UnterminatedString(start)),
                }
                Ok(())
            }
            ch if ch == '\u{09}' || ('\u{20}' <= ch && ch <= '\u{10ffff}' && ch != '\u{7f}') => {
                val.push(ch);
                Ok(())
            }
            _ => Err(Error::InvalidCharInString(i, ch)),
        })
    }

    fn whitespace(&mut self, start: usize) -> Token<'a> {
        while self.eat(' ') || self.eat('\t') {
            // eat
        }
        WhiteSpace(&self.src[start..self.current()])
    }

    fn comment(&mut self, start: usize) -> Token<'a> {
        while let Some((_, ch)) = self.chars.clone().next() {
            if ch != '\t' && (ch < '\u{20}' || ch > '\u{10ffff}') {
                break;
            }
            self.get_next_char();
        }
        Comment(&self.src[start..self.current()])
    }

    fn literal_string(&mut self, start: usize) -> Result<Token<'a>, Error> {
        self.read_string(
            '\'',
            start,
            &mut |_tokenizer: &mut Tokenizer<'_>, val, _multi, i, ch: char| {
                if ch == '\u{09}' || ('\u{20}' <= ch && ch <= '\u{10ffff}' && ch != '\u{7f}') {
                    val.push(ch);
                    Ok(())
                } else {
                    Err(Error::InvalidCharInString(i, ch))
                }
            },
        )
    }

    fn hex(&mut self, start: usize, i: usize, len: usize) -> Result<char, Error> {
        let mut buf = StdString::with_capacity(len);
        for _ in 0..len {
            match self.get_next_char() {
                Some((_, ch)) if ch as u32 <= 0x7F && ch.is_digit(16) => buf.push(ch),
                Some((i, ch)) => return Err(Error::InvalidHexEscape(i, ch)),
                None => return Err(Error::UnterminatedString(start)),
            }
        }
        let val = u32::from_str_radix(&buf, 16).unwrap();
        match char::from_u32(val) {
            Some(ch) => Ok(ch),
            None => Err(Error::InvalidEscapeValue(i, val)),
        }
    }

    fn calculate_span(&mut self, start: usize) -> Span {
        let end = self
            .peek_next()
            .map(|t| t.0)
            .unwrap_or_else(|| self.src.len());
        Span { start, end }
    }

    fn peek_next(&mut self) -> Option<(usize, char)> {
        self.chars.clone().clone().next()
    }

    fn is_last_char(&mut self) -> bool {
        self.chars.clone().next().is_none()
    }
}

impl MaybeString {
    fn push(&mut self, c: char) {
        match *self {
            MaybeString::NotEscaped(..) => {}
            MaybeString::Owned(ref mut string) => string.push(c),
        }
    }

    fn owned(&mut self, src: &str) {
        match *self {
            MaybeString::NotEscaped(start) => *self = MaybeString::Owned(src[start..].to_owned()),
            MaybeString::Owned(..) => {}
        }
    }

    fn into_cow(self, src: &str) -> Cow<'_, str> {
        match self {
            MaybeString::NotEscaped(start) => Cow::Borrowed(&src[start..]),
            MaybeString::Owned(string) => Cow::Owned(string),
        }
    }
}

impl<'a> Token<'a> {
    pub fn describe(&self) -> &'static str {
        match *self {
            Token::Keylike(_) => "an identifier",
            Token::Equals => "an equals",
            Token::Period => "a period",
            Token::Comment(_) => "a comment",
            Token::NewLine => "a newline",
            Token::WhiteSpace(_) => "whitespace",
            Token::Comma => "a comma",
            Token::RightBrace => "a right brace",
            Token::LeftBrace => "a left brace",
            Token::RightBracket => "a right bracket",
            Token::LeftBracket => "a left bracket",
            Token::String { multiline, .. } => {
                if multiline {
                    "a multiline string"
                } else {
                    "a string"
                }
            }
            Token::Colon => "a colon",
            Token::Plus => "a plus",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Span, Token, Tokenizer};

    #[test]
    fn should_parse_toml() {
        let toml_file_content = r#"
    title = "TOML Example"

    [owner]
    name = "Tom Preston-Werner"
    dob = 1979-05-27T07:32:00-08:00
    
    [database]
    enabled = true
    ports = [ 8000, 8001, 8002 ]
    data = [ ["delta", "phi"], [3.14] ]
    temp_targets = { cpu = 79.5, case = 72.0 }
    
    [servers]
    
    [servers.alpha]
    ip = "10.0.0.1"
    role = "frontend"
    
    [servers.beta]
    ip = "10.0.0.2"
    role = "backend"
    "#;

        let mut tokenizer = Tokenizer::new(toml_file_content);
        let mut tokens: Vec<(Span, Token<'_>)> = vec![];

        while !tokenizer.is_last_char() {
            let token = tokenizer.tokenize();

            match token {
                Ok(t) => match t {
                    Some(value) => tokens.push(value),
                    None => {}
                },
                Err(_) => {}
            }
        }

        println!("{:?}", tokens);
    }
}

use std::marker::PhantomData;

pub fn parse(_input: &str) -> Result<(), ParseError> {
  Ok(())
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    GenericError,
    ExpectingCharacter(char),
    ExpectingPredicate,
    ExpectingOneOfToParse,
    EndOfInput,
}

pub trait Parser<'a, T> {
    fn parse(&self, input: &'a str) -> Result<(T, &'a str), ParseError>;
}

impl <'a, T, F> Parser<'a, T> for F where F: Fn(&'a str) -> Result<(T, &'a str), ParseError> {
    fn parse(&self, input: &'a str) -> Result<(T, &'a str), ParseError> {
        self(input)
    }
}

pub struct Character {
  character_to_match: char,
}

impl<'a> Parser<'a, char> for Character {
    fn parse(&self, input: &'a str) -> Result<(char, &'a str), ParseError> {
        if input.starts_with(self.character_to_match) {
            Ok((self.character_to_match, &input[1..]))
        } else {
            Err(ParseError::ExpectingCharacter(self.character_to_match))
        }
    }
}

impl Character {
    pub fn new<'a>(character_to_match: char) -> impl Parser<'a, char> {
        Self { character_to_match }
    }
}

pub fn character<'a>(character_to_match: char) -> impl Parser<'a, char> {
  Character::new(character_to_match)
}

pub struct Any<F> where F: Fn(char) -> bool + Sized {
    predicate: F,
}

impl<'a, F> Parser<'a, char> for Any<F> where F: Fn(char) -> bool + Sized {
    fn parse(&self, input: &'a str) -> Result<(char, &'a str), ParseError> {
        let character = input.chars().next();
        match character {
            Some(c) => {
                if (self.predicate)(c) {
                    Ok((c, &input[1..]))
                } else {
                    Err(ParseError::ExpectingPredicate)
                }
            },

            None => {
                Err(ParseError::EndOfInput)
            }
        }
    }
}

impl<F> Any<F> where F: Fn(char) -> bool + Sized {
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

pub fn any<'a, F>(predicate: F) -> impl Parser<'a, char> where F: Fn(char) -> bool + Sized {
    Any::new(predicate)
}

pub struct Map<'a, I, O, P, F> where I: 'a, P: Parser<'a, I> + Sized, F: Fn(I) -> O + Sized {
    parser: P,
    map: F,
    phantom: PhantomData<&'a I>,
}

impl<'a, I, O, P, F> Parser<'a, O> for Map<'a, I, O, P, F> where I: 'a, P: Parser<'a, I> + Sized, F: Fn(I) -> O + Sized {
    fn parse(&self, input: &'a str) -> Result<(O, &'a str), ParseError> {
        let attempt = self.parser.parse(input);
        attempt.map(|(v, rest)|{ ((self.map)(v), rest)})
    }
}

impl<'a, I, O, P, F> Map<'a, I, O, P, F> where I: 'a, P: Parser<'a, I> + Sized, F: Fn(I) -> O + Sized {
    pub fn new(parser: P, map: F) -> Self {
        Self { parser, map, phantom: PhantomData }
    }
}

pub fn map<'a, I, O, P, F>(parser: P, map: F) -> impl Parser<'a, O> where I: 'a, P: Parser<'a, I> + Sized, F: Fn(I) -> O + Sized {
    Map::new(parser, map)
}

pub struct Between<'a, T, P> where T: 'a, P: Parser<'a, T> + Sized {
    lower_limit: u8,
    upper_limit: Limit,
    parser: P,
    phantom: PhantomData<&'a T>,
}

pub enum Limit {
  At(u8),
  Infinity,
}

impl Limit {
  pub fn is_bigger_then(&self, n: u8) -> bool {
    match self {
      Limit::At(threshold) => threshold > &n,

      Limit::Infinity => true,
    }
  }
}

impl<'a, T, P> Parser<'a, Vec<T>> for Between<'a, T, P> where P: Parser<'a, T> + Sized {
    fn parse(&self, input: &'a str) -> Result<(Vec<T>, &'a str), ParseError> {
        let mut result = vec![];
        let mut source = input;
        let mut count = 0;
        while count < self.lower_limit {
            let attempt = self.parser.parse(source);
            match attempt {
                Ok((value, rest)) => {
                    result.push(value);
                    source = rest;
                }

                Err(e) => {
                    return Err(e);
                }
            }
            count += 1;
        }
        while self.upper_limit.is_bigger_then(count) {
            let attempt = self.parser.parse(source);
            match attempt {
                Ok((value, rest)) => {
                    result.push(value);
                    source = rest;
                }

                Err(_) => {
                    break;
                }
            }
            count += 1;
        }
        Ok((result, source))
    }
}

impl<'a, T, P> Between<'a, T, P> where T: 'a, P: Parser<'a, T> + Sized {
    pub fn new(lower_limit: u8, upper_limit: Limit, parser: P) -> Self {
        Self { lower_limit, upper_limit, parser, phantom: PhantomData }
    }
}

pub fn between<'a, T>(lower_limit: u8, upper_limit: u8, parser: impl Parser<'a, T>) -> impl Parser<'a, Vec<T>> where T: 'a {
    Between::new(lower_limit, Limit::At(upper_limit), parser)
}

pub fn at_least<'a, T>(lower_limit: u8, parser: impl Parser<'a, T>) -> impl Parser<'a, Vec<T>> where T: 'a {
    Between::new(lower_limit, Limit::Infinity, parser)
}

pub fn many<'a, T>(parser: impl Parser<'a, T>) -> impl Parser<'a, Vec<T>> where T: 'a {
    at_least(0, parser)
}

pub struct OneOf<'a, T, P> where T: 'a, P: Parser<'a, T> + Sized {
    options: Vec<P>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T, P> Parser<'a, T> for OneOf<'a, T, P> where T: 'a, P: Parser<'a, T> + Sized {
    fn parse(&self, input: &'a str) -> Result<(T, &'a str), ParseError> {
        for ref parser in &self.options {
            let attempt = parser.parse(input);
            if attempt.is_ok() {
                return attempt
            }
        }
        Err(ParseError::ExpectingOneOfToParse)
    }
}

impl<'a, T, P> OneOf<'a, T, P> where T: 'a, P: Parser<'a, T> + Sized {
    pub fn new(options: Vec<P>) -> Self {
        Self { options, phantom: PhantomData }
    }
}

pub fn one_of<'a, T, P>(options: Vec<P>) -> impl Parser<'a, T> where T: 'a, P: Parser<'a, T> + Sized {
    OneOf::new(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a_character() {
        let input = "ABCD";
        let parser = character('A');

        let actual = parser.parse(input);

        let expected = Ok(('A', "BCD"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_any_digit() {
        let input = "0123";
        let parser = any(|c: char| c.is_ascii_digit());

        let actual = parser.parse(input);

        let expected = Ok(('0', "123"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_any_digit_as_number() {
        let input = "1230";
        let parser = map(
          any(|c: char| c.is_ascii_digit()), 
          |c: char| c.to_digit(10).unwrap_or(0));

        let actual = parser.parse(input);

        let expected = Ok((1, "230"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_between_2_and_4_digits() {
        let input = "12345";
        let parser = between(2, 4, any(|c: char| c.is_ascii_digit()));

        let actual = parser.parse(input);

        let expected = Ok((vec!['1', '2', '3', '4'], "5"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_one_of_a_or_b() {
        let input = "a1";
        let parser = one_of(vec![character('a'), character('b')]);

        let actual = parser.parse(input);

        let expected = Ok(('a', "1"));
        assert_eq!(actual, expected);
    }
}
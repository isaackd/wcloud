use std::collections::HashMap;
use std::collections::HashSet;
use regex::{Regex, Matches, Match};

pub mod sat;

pub struct WordOptions<'a> {
    filter: HashSet<&'a str>,
    min_word_length: u32,
    exclude_numbers: bool,
    max_words: u32,
}

impl<'a> Default for WordOptions<'a> {
    fn default() -> Self {
        WordOptions {
            filter: HashSet::new(),
            min_word_length: 0,
            exclude_numbers: true,
            max_words: 200,
        }
    }
}

impl<'a> WordOptions<'a> {
    // TODO: I don't even know if this is idiomatic at this point, but it works!
    fn apply<'b>(&'b self, matches: Matches<'b, 'b>) -> Box<dyn Iterator<Item = Match<'b>> + 'b> {
        let mut result: Box<dyn Iterator<Item = Match<'b>> + 'b> = Box::new(matches);

        if self.max_words != 0 {
            result = Box::new(result.take(self.max_words as usize));
        }
        if !self.filter.is_empty() {
            result = Box::new(result.filter(move |word| !self.filter.contains(word.as_str())));
        }
        if self.min_word_length > 0 {
            result = Box::new(result.filter(move |word| word.as_str().len() >= self.min_word_length as usize));
        }
        if self.exclude_numbers {
            result = Box::new(result.filter(move |word| !word.as_str().chars().all(char::is_numeric)));
        }

        result
    }
}

pub fn get_word_frequencies<'a>(text: &'a str, word_regex: &'a Regex, options: &'a WordOptions<'a>) -> (HashMap<&'a str, usize>, usize) {
    let mut frequencies = HashMap::new();
    let mut max_freq = 0;

    let included_words = options.apply(word_regex.find_iter(text));

    for word in included_words {
        let entry = frequencies.entry(word.as_str()).or_insert(0);
        *entry += 1;

        if *entry > max_freq {
            max_freq = *entry;
        }
    }

    (frequencies, max_freq)
}

pub fn get_normalized_word_frequencies<'a, 'b>(text: &'a str, word_regex: &'a Regex, options: &'a WordOptions<'a>) -> Vec<(&'a str, f32)> {
    let (frequencies, max_freq) = get_word_frequencies(text, &word_regex, &options);

    let mut normalized: Vec<(&str, f32)> = frequencies.iter().map(|(key, val)| {
        (*key, *val as f32 / max_freq as f32)
    }).collect();

    normalized.sort_unstable_by(|a, b| {
        (b.1).partial_cmp(&a.1).unwrap()
    });
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_word_frequencies() {
        let pat = Regex::new("\\w[\\w']*").unwrap();
        let words = "A woodchuck would chuck as much wood as a woodchuck could chuck if a woodchuck could chuck wood";

        let options = WordOptions::default();
        let frequencies = get_word_frequencies(words, &pat, &options);

        let expected: HashMap<&str, usize> = vec![
            ("if", 1), ("a", 2), ("chuck", 3),
            ("would", 1), ("woodchuck", 3), ("A", 1),
            ("as", 2), ("could", 2), ("much", 1),
            ("wood", 2)
        ].into_iter().collect();

        assert_eq!(frequencies.0, expected);
        assert_eq!(frequencies.1, 3);
    }
}

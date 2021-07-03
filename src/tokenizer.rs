use regex::{Regex, Match};
use std::collections::{HashSet, HashMap};

// TODO: Use lazy_static or PHF to make this a HashSet?
pub const DEFAULT_EXCLUDE_WORDS_TEXT: &str = include_str!("../exclude_words.txt");

pub struct Tokenizer {
    pub regex: Regex,
    filter: HashSet<String>,
    pub min_word_length: u32,
    pub exclude_numbers: bool,
    pub max_words: u32,
    pub repeat: bool,
}

impl<'a> Default for Tokenizer {
    fn default() -> Self {
        let regex = Regex::new("\\w[\\w']*")
            .expect("Unable to compile tokenization regex");

        Tokenizer {
            regex,
            filter: HashSet::new(),
            min_word_length: 0,
            exclude_numbers: true,
            max_words: 200,
            repeat: false,
        }
    }
}

impl<'a> Tokenizer {
    fn tokenize(&'a self, text: &'a str) -> Box<dyn Iterator<Item=Match<'a>> + 'a> {
        let mut result: Box<dyn Iterator<Item=Match<'a>> + 'a>
            = Box::new(self.regex.find_iter(text));

        if self.max_words != 0 {
            result = Box::new(result.take(self.max_words as usize));
        }
        if !self.filter.is_empty() {
            result = Box::new(result.filter(move |word| {
                let word_lower = word.as_str().to_lowercase();
                !self.filter.contains(word_lower.as_str())
            }));
        }
        if self.min_word_length > 0 {
            result = Box::new(result.filter(move |word| word.as_str().len() >= self.min_word_length as usize));
        }
        if self.exclude_numbers {
            result = Box::new(result.filter(move |word| !word.as_str().chars().all(char::is_numeric)));
        }

        result
    }

    fn keep_common_case(map: &HashMap<&'a str, usize>) -> HashMap<&'a str, usize> {
        type WordCount<'a> = HashMap<&'a str, usize>;
        let mut common_cases = HashMap::<String, WordCount>::new();
        for &key in map.keys() {
            common_cases.entry(key.to_lowercase()).or_default();
        }

        for (key, val) in map {
            let key_lower = key.to_lowercase();
            common_cases.get_mut(&key_lower)
                .unwrap()
                .insert(key, *val);
        }

        common_cases.values().map(|val| {
            let mut most_common_case: Vec<(&str, usize)> = val.iter().map(|(case_key, case_val)| {
                (*case_key, *case_val)
            }).collect();

            most_common_case.sort_by(|a, b| {
                if a.1 != b.1 {
                    (b.1).partial_cmp(&a.1).unwrap()
                }
                else {
                    (b.0).partial_cmp(a.0).unwrap()
                }
            });

            let occurrence_sum = val.values().sum();

            (most_common_case.first().unwrap().0, occurrence_sum)
        }).collect()
    }

    fn get_word_frequencies(&'a self, text: &'a str) -> (HashMap<&'a str, usize>, usize) {
        let mut frequencies = HashMap::new();

        let included_words = self.tokenize(text);

        for word in included_words {
            let entry = frequencies.entry(word.as_str()).or_insert(0);
            *entry += 1;
        }

        let common_cased_map = Self::keep_common_case(&frequencies);
        let max_freq = common_cased_map.values().max()
            .expect("Can't get max frequency")
            .clone();

        (common_cased_map, max_freq)
    }

    pub fn get_normalized_word_frequencies(&'a self, text: &'a str) -> Vec<(&'a str, f32)> {
        let (frequencies, max_freq) = self.get_word_frequencies(text);

        if frequencies.is_empty() {
            return Vec::new();
        }

        let mut normalized_freqs: Vec<(&str, f32)> = frequencies.iter().map(|(key, val)| {
            (*key, *val as f32 / max_freq as f32)
        }).collect();

        normalized_freqs.sort_by(|a, b| {
            if a.1 != b.1 {
                (b.1).partial_cmp(&a.1).unwrap()
            }
            else {
                (a.0).partial_cmp(b.0).unwrap()
            }
        });

        if self.repeat && normalized_freqs.len() < self.max_words as usize {
            let times_extend = ((self.max_words as f32 / normalized_freqs.len() as f32).ceil()) as u32 - 1;
            // println!("Times extend: {}, max_words: {}, freqs len: {}", times_extend, self.max_words, normalized_freqs.len());
            let freqs_clone = normalized_freqs.clone();
            let down_weight = normalized_freqs.last()
                .expect("The normalized frequencies vec is empty")
                .1;

            for i in 1..=times_extend {
                normalized_freqs.extend(
                    freqs_clone.iter().map(|(word, freq)| {
                        (*word, freq * down_weight.powf(i as f32))
                    })
                )
            }
        }

        normalized_freqs
    }

    pub fn with_regex(mut self, value: Regex) -> Self {
        self.regex = value;
        self
    }
    pub fn with_filter(mut self, value: HashSet<&str>) -> Self {
        self.filter = value.iter()
            .map(|el| el.to_lowercase())
            .collect();
        self
    }
    pub fn with_min_word_length(mut self, value: u32) -> Self {
        self.min_word_length = value;
        self
    }
    pub fn with_exclude_numbers(mut self, value: bool) -> Self {
        self.exclude_numbers = value;
        self
    }
    pub fn with_max_words(mut self, value: u32) -> Self {
        self.max_words = value;
        self
    }
    pub fn with_repeat(mut self, value: bool) -> Self {
        self.repeat = value;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_word_frequencies() {
        let words = "A woodchuck would chuck as much wood as a woodchuck could chuck if a woodchuck could chuck wood";

        let tokenizer = Tokenizer::default();
        let frequencies = tokenizer.get_word_frequencies(words);

        let expected: HashMap<&str, usize> = vec![
            ("could", 2), ("much", 1), ("if", 1), ("woodchuck", 3),
            ("as", 2), ("wood", 2), ("would", 1), ("chuck", 3), ("a", 3)
        ].into_iter().collect();

        assert_eq!(frequencies.0, expected);
        assert_eq!(frequencies.1, 3);
    }

    #[test]
    fn simple_normalized_word_frequencies() {
        let words = "A a wood chuck could could Could ChuCK";

        let tokenizer = Tokenizer::default()
            .with_repeat(true)
            .with_max_words(12);
        let frequencies = tokenizer.get_normalized_word_frequencies(words);

        let expected = vec![
            ("could", 1.0), ("a", 0.6666667), ("chuck", 0.6666667), ("wood", 0.33333334),
            ("could", 0.33333334), ("a", 0.22222224), ("chuck", 0.22222224), ("wood", 0.11111112),
            ("could", 0.11111112), ("a", 0.07407408), ("chuck", 0.07407408), ("wood", 0.03703704)
        ];

        assert_eq!(frequencies, expected);
    }

    #[test]
    fn keeps_most_common_case() {
        let words = "LUKE Luke luke luke Luke LUKE LUKE lUKE Luke LUKE luKe lukE";

        let tokenizer = Tokenizer::default();
        let frequencies = tokenizer.get_word_frequencies(words);

        let expected: HashMap<&str, usize> = vec![
            ("LUKE", 12)
        ].into_iter().collect();

        assert_eq!(frequencies.0, expected);
    }

    #[test]
    fn filter_works() {
        let words = "The quick brown fox jumps over the lazy dog. The dog was otherwise very fine.";
        let filter = DEFAULT_EXCLUDE_WORDS_TEXT
            .split("\n")
            .collect::<HashSet<_>>();

        let tokenizer = Tokenizer::default()
            .with_filter(filter);
        let frequencies = tokenizer.get_word_frequencies(words);

        println!("original words: {:?} changed: {:?}", words, frequencies.0);

        let expected: HashMap<&str, usize> = vec![
            ("fox", 1), ("brown", 1), ("dog", 2), ("lazy", 1), ("jumps", 1), ("fine", 1), ("quick", 1)
        ].into_iter().collect();

        assert_eq!(frequencies.0, expected);
    }
}

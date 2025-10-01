// src/punctuation.rs
#![allow(dead_code)] // Silences warnings for unused items for now

use fancy_regex::Regex;
use lazy_static::lazy_static;

// Import necessary constants from your char_classes module
// Using aliases for brevity, matching common spaCy internal naming.
use crate::char_classes::{
    FINAL_ALPHA_CONTENT_STR as ALPHA,
    FINAL_ALPHA_LOWER_CONTENT_STR as ALPHA_LOWER,
    FINAL_ALPHA_UPPER_CONTENT_STR as ALPHA_UPPER,
    FINAL_ALPHANUM_CONTENT_STR as ALPHANUM, // Assuming this is defined in char_classes
    DIGITS_CONTENT_STR as DIGITS,           // Assuming this is defined in char_classes
    COMBINING_DIACRITICS_CONTENT_STR as COMBINING_DIACRITICS,
    CONCAT_QUOTES_CONTENT_STR as CONCAT_QUOTES,
    CURRENCY_PATTERN_PART as CURRENCY, // This is the "€|$|..." string
    HYPHENS_PATTERN_PART as HYPHENS,   // This is the "-|–|..." string
    PUNCT_PATTERN_PART as PUNCT,       // This is the ".|;|..." string
    UNITS_PATTERN_PART as UNITS,       // This is the "km|m²|..." string
    LIST_CURRENCY_STRS as LIST_CURRENCY, // This is an &[&str]
    LIST_ELLIPSES_STRS as LIST_ELLIPSES, // This is an &[&str]
    LIST_ICONS_PATTERNS as LIST_ICONS, // This is an &[&str] (list of icon regex patterns)
    LIST_PUNCT_STRS as LIST_PUNCT,       // This is an &[&str]
    LIST_QUOTES_STRS as LIST_QUOTES,     // This is an &[&str]
    // You might also have these defined in char_classes.rs, or define them here.
    // For this example, let's assume they are string literals here.
    // TOKEN_MATCH_PATTERN_STR,
    // URL_MATCH_PATTERN_STR,
};

// Regex patterns defined directly for token_match and url_match
// These could also come from char_classes.rs if you prefer
const TOKEN_MATCH_PATTERN_STR: &str = r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*";
const URL_MATCH_PATTERN_STR: &str = r"(?:https?://|www\.)[^\s/$.?#].*?[^\s]*";


lazy_static! {
    /// English tokenizer prefixes
    /// These are regex patterns that should be matched at the beginning of a token.
    pub static ref TOKENIZER_PREFIXES: Vec<String> = {
        // spaCy's default prefixes often include specific characters and patterns from char_classes.
        // This list is based on common English prefixes and the structure of spaCy's rules.
        let mut v: Vec<String> = vec![
            // Characters like §, %, =, —, –, plus that is not followed by a digit
            r"^[§%=–—]".to_string(), // Using a character class for single chars
            r"^\+(?!\d)".to_string(), // Plus not followed by digit
            // Opening brackets and quotes (using char_classes constants)
            // Assuming OPEN_QUOTES_PATTERN_PART and CURRENCY_PATTERN_PART are defined in char_classes
            // and are suitable for direct prefix matching (e.g. already char classes or specific patterns)
            format!(r"^{}", crate::char_classes::OPEN_QUOTES_PATTERN_PART),
            r"^[(\[{<]".to_string(),
            format!(r"^(?:{})", CURRENCY), // CURRENCY is "€|$|..."
        ];
        // Extend with lists of specific punctuation, icons etc. that can be prefixes
        v.extend(LIST_PUNCT.iter().map(|&p| format!(r"^{}", regex::escape(p)))); // Escape literals
        v.extend(LIST_ELLIPSES.iter().map(|&p| format!(r"^{}", p))); // Ellipses are regex
        v.extend(LIST_QUOTES.iter().map(|&p| format!(r"^{}", regex::escape(p))));
        // LIST_ICONS are already regex patterns
        v.extend(LIST_ICONS.iter().map(|p_str| format!(r"^{}", p_str)));
        v
    };

    /// English tokenizer suffixes
    /// These are regex patterns that should be matched at the end of a token.
    pub static ref TOKENIZER_SUFFIXES: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        // Simple punctuation, ellipses, quotes, icons
        v.extend(LIST_PUNCT.iter().map(|&p| format!(r"{}$", regex::escape(p))));
        v.extend(LIST_ELLIPSES.iter().map(|&p| format!(r"{}$", p)));
        v.extend(LIST_QUOTES.iter().map(|&p| format!(r"{}$", regex::escape(p))));
        v.extend(LIST_ICONS.iter().map(|p_str| format!(r"{}$", p_str))); // LIST_ICONS are regex

        // Specific suffixes from spaCy's English rules
        v.extend(vec![
            r"'s$", r"'S$", r"’s$", r"’S$", // Possessives
            r"—$", r"–$", // Dashes
            r"(?i:[dn])'(?:t|ve|s|m|re|ll|d)$", // Common contractions like n't, 've
        ].into_iter().map(String::from));

        // Suffixes with lookbehinds
        v.push(r"(?<=\d)\+$".to_string()); // Plus preceded by a digit
        v.push(r"(?<=°[FfCcKk])\.$".to_string()); // Period after temperature units

        // CURRENCY and UNITS are "€|$|..." and "km|m²|..." respectively
        v.push(format!(r"(?<=\d)(?:{})$", CURRENCY));
        v.push(format!(r"(?<=\d)(?:{})$", UNITS)); // Units after a digit

        // Complex period suffix: e.g., ". after number/alpha + specific symbols"
        // Python: r"(?<=[0-9{}{}(?:{})])\." .format(ALPHA_LOWER, "%²\\-\\+", CONCAT_QUOTES, PUNCT)
        // This needs careful construction. ALPHA_LOWER and CONCAT_QUOTES are char class content.
        // PUNCT is an alternation pattern.
        // The middle part "%²\\-\\+" needs to be a raw string in Rust: "%²\\\\-\\\\+"
        let complex_suffix_punct_chars = format!("{}{}{}", DIGITS, ALPHA_LOWER, CONCAT_QUOTES); // Content for inside []
        let complex_suffix_middle_literals = "%²\\-\\+"; // Literals, ensure correct escaping for regex
        v.push(format!(
            r"(?<=[{}{}](?:{}))\.$",
            complex_suffix_punct_chars, // This goes inside the outer []
            regex::escape(complex_suffix_middle_literals), // Escape this part
            PUNCT // PUNCT is already "…| Battleship\. …|,"
        ));

        // Period after two uppercase letters (e.g., U.S.)
        v.push(format!(r"(?<=[{}][{}]\.)$", ALPHA_UPPER, ALPHA_UPPER)); // ALPHA_UPPER is content for []
        v
    };

    /// English tokenizer infixes
    /// These are regex patterns that split a token internally.
    pub static ref TOKENIZER_INFIXES: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        v.extend(LIST_ELLIPSES.iter().map(|s| s.to_string())); // Ellipses are already regex
        v.extend(LIST_ICONS.iter().map(|s| s.to_string()));    // Icons are already regex

        // Patterns from spaCy's English infixes
        v.push(r"(?<=\d)[+\-*^](?=\d|-)".to_string()); // Arithmetic ops/hyphen
        v.push(format!( // ALPHA_LOWER, CONCAT_QUOTES, ALPHA_UPPER are content for []
            r"(?<=[{}{}])\.(?=[{}{}])",
            ALPHA_LOWER, CONCAT_QUOTES, ALPHA_UPPER, CONCAT_QUOTES
        ));
        v.push(format!( // ALPHA is content for []
            r"(?<=[{}]),(?=[{}])",
            ALPHA, ALPHA
        ));
        // Python: r"(?<=[{a}0-9])(?:{h})(?=[{a}])".format(a=ALPHA, h=HYPHENS)
        // HYPHENS is an alternation pattern like "-|–|..."
        // ALPHA and DIGITS are content for [].
        v.push(format!(
            r"(?<=[{}{}{}\d])(?:{})(?=[{}])", // Use ALPHANUM which is ALPHA + DIGITS content
            ALPHA, DIGITS, // For the lookbehind part to include digits with ALPHA
            HYPHENS, // HYPHENS is the pattern part like "-|–|..."
            ALPHA
        ));
        v.push(format!( // ALPHA is content for [].
            r"(?<=[{}{}\d])(?:[:<>=/])(?=[{}])", // Use ALPHANUM
            ALPHA, DIGITS,
            ALPHA
        ));
        v
    };

    // Compiled Regex for Token Match and URL Match
    pub static ref TOKEN_MATCH_REGEX: Option<Regex> = Regex::new(TOKEN_MATCH_PATTERN_STR).ok();
    pub static ref URL_MATCH_REGEX: Option<Regex> = Regex::new(URL_MATCH_PATTERN_STR).ok();


    // Combining Diacritics versions (if needed, these clone and add more specific rules)
    pub static ref COMBINING_DIACRITICS_TOKENIZER_SUFFIXES: Vec<String> = {
        let mut v = TOKENIZER_SUFFIXES.clone();
        v.push(format!( // ALPHA and COMBINING_DIACRITICS are content for []
            r"(?<=[{}][{}])\.",
            ALPHA, COMBINING_DIACRITICS
        ));
        v
    };
    pub static ref COMBINING_DIACRITICS_TOKENIZER_INFIXES: Vec<String> = {
        let mut v = TOKENIZER_INFIXES.clone();
        v.push(format!(
            r"(?<=[{}][{}])\.(?=[{}{}])",
            ALPHA_LOWER, COMBINING_DIACRITICS, ALPHA_UPPER, CONCAT_QUOTES
        ));
        v.push(format!(
            r"(?<=[{}][{}]),(?=[{}])",
            ALPHA, COMBINING_DIACRITICS, ALPHA
        ));
        v.push(format!( // HYPHENS is pattern part
            r"(?<=[{}][{}])(?:{})(?=[{}])",
            ALPHA, COMBINING_DIACRITICS, HYPHENS, ALPHA
        ));
        v.push(format!(
            r"(?<=[{}][{}])(?:[:<>=/])(?=[{}])",
            ALPHA, COMBINING_DIACRITICS, ALPHA
        ));
        v
    };
}

// Example function to demonstrate usage (optional)
pub fn print_patterns() {
    println!("--- Tokenizer Prefixes ---");
    for p in TOKENIZER_PREFIXES.iter() { println!("{}", p); }
    println!("\n--- Tokenizer Suffixes ---");
    for s in TOKENIZER_SUFFIXES.iter() { println!("{}", s); }
    println!("\n--- Tokenizer Infixes ---");
    for i in TOKENIZER_INFIXES.iter() { println!("{}", i); }
    if let Some(re) = TOKEN_MATCH_REGEX.as_ref() {
        println!("\n--- Token Match Pattern --- \n{}", re.as_str());
    }
    if let Some(re) = URL_MATCH_REGEX.as_ref() {
        println!("\n--- URL Match Pattern --- \n{}", re.as_str());
    }
}
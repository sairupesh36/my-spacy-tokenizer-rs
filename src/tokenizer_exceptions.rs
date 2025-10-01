// tokenizer_exceptions.rs

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

/// Symbolic keys for token attributes
pub const ORTH: &str = "ORTH";
pub const NORM: &str = "NORM";

// Import character classes from your char_classes.rs (you must implement this)
// pub const ALPHA_LOWER: &str = ...;

lazy_static! {
    /// URL pattern regex (faithful to the Python version)
    pub static ref URL_PATTERN: &'static str = concat!(
        r"^",
        r"(?:(?:[\\w\\+\\-\\.]{2,})://)?",
        r"(?:\\S+(?::\\S*)?@)?",
        r"(?:",
            r"(?!(?:10|127)(?:\\.\\d{1,3}){3})",
            r"(?!(?:169\\.254|192\\.168)(?:\\.\\d{1,3}){2})",
            r"(?!172\\.(?:1[6-9]|2\\d|3[0-1])(?:\\.\\d{1,3}){2})",
            r"(?:[1-9]\\d?|1\\d\\d|2[01]\\d|22[0-3])",
            r"(?:\\.(?:1?\\d{1,2}|2[0-4]\\d|25[0-5])){2}",
            r"(?:\\.(?:[1-9]\\d?|1\\d\\d|2[0-4]\\d|25[0-4]))",
            r"|",
            r"(?:(?:(?:[A-Za-z0-9\\u00a1-\\uffff])",
            r"[A-Za-z0-9\\u00a1-\\uffff_-]{0,62})?",
            r"[A-Za-z0-9\\u00a1-\\uffff]\\.)+",
            r"(?:[", // -- ALPHA_LOWER inserted at runtime
            // see below!
    );
    pub static ref URL_REGEX: Regex = Regex::new(&{
        let mut pat = URL_PATTERN.to_string();
        // Insert ALPHA_LOWER at runtime for the TLD section.
        pat.push_str(crate::char_classes::ALPHA_LOWER); // ALPHA_LOWER must be defined in your char_classes.rs
        pat.push_str("]{2,63})");
        pat.push_str(
            r")(?::\d{2,5})?(?:[/?#]\S*)?$"
        );
        pat
    }).unwrap();
}

/// Test if a string matches the URL pattern.
pub fn url_match(s: &str) -> bool {
    URL_REGEX.is_match(s)
}

// ========== BASE_EXCEPTIONS setup ==========

/// Exception data structure (simple, extensible)
#[derive(Clone, Debug)]
pub struct ExceptionToken {
    pub orth: String,
    pub norm: Option<String>,
}

/// HashMap for exceptions
pub type ExceptionMap = HashMap<String, Vec<ExceptionToken>>;

lazy_static! {
    pub static ref BASE_EXCEPTIONS: ExceptionMap = {
        let mut m = ExceptionMap::new();

        // 1. Special whitespace and dashes
        m.insert(" ".into(), vec![ExceptionToken { orth: " ".into(), norm: None }]);
        m.insert("\t".into(), vec![ExceptionToken { orth: "\t".into(), norm: None }]);
        m.insert("\\t".into(), vec![ExceptionToken { orth: "\\t".into(), norm: None }]);
        m.insert("\n".into(), vec![ExceptionToken { orth: "\n".into(), norm: None }]);
        m.insert("\\n".into(), vec![ExceptionToken { orth: "\\n".into(), norm: None }]);
        m.insert("\u{2014}".into(), vec![ExceptionToken { orth: "\u{2014}".into(), norm: None }]);
        m.insert("\u{00a0}".into(), vec![ExceptionToken { orth: "\u{00a0}".into(), norm: Some("  ".into()) }]);

        // 2. Orth-only forms (quotes, C++, a. ... z., ä., ö., ü.)
        for orth in &[
            "'", "\\\")", "<space>", "''", "C++", "a.", "b.", "c.", "d.", "e.", "f.", "g.", "h.",
            "i.", "j.", "k.", "l.", "m.", "n.", "o.", "p.", "q.", "r.", "s.", "t.", "u.", "v.",
            "w.", "x.", "y.", "z.", "ä.", "ö.", "ü.",
        ] {
            m.insert((*orth).into(), vec![ExceptionToken { orth: (*orth).into(), norm: None }]);
        }

        m
    };
}
// ...continuation from PART 1...

lazy_static! {
    pub static ref BASE_EXCEPTIONS: ExceptionMap = {
        let mut m = ExceptionMap::new();

        // (Already defined: whitespace and orth-only forms...)

        // --- Emoticons (each maps to itself as a single token) ---
        let emoticons = vec![
            ":)", ":-)", ":))", ":-))", ":)))", ":-)))", "(:", "(-:", "=)", "(=", ":]", ":-]", "[:", "[-:", "[=", "=]", ":o)", "(o:", ":}", ":-}", "8)", "8-)", "(-8", ";)", ";-)", "(;", "(-;", ":(", ":-(", ":((", ":-((", ":(((", ":-(((", "):", ")-:", "=(", ">:(", ":')", ":'-)", ":'(", ":'-(", ":/", ":-/", "=/", "=|", ":|", ":-|", "]=", "=[", ":1", ":P", ":-P", ":p", ":-p", ":O", ":-O", ":o", ":-o", ":0", ":-0", ":()", ">:o", ":*", ":-*", ":3", ":-3", "=3", ":>", ":- >", ":X", ":-X", ":x", ":-x", ":D", ":-D", ";D", ";-D", "=D", "xD", "XD", "xDD", "XDD", "8D", "8-D", "^_^", "^__^", "^___^", ">.<", ">.>", "<.<", "._.", ";_;", "-_-", "-__-", "v.v", "V.V", "v_v", "V_V", "o_o", "o_O", "O_o", "O_O", "0_o", "o_0", "0_0", "o.O", "O.o", "O.O", "o.o", "0.0", "o.0", "0.o", "@_@", "<3", "<33", "<333", "</3", "(^_^)", "(-_-)", "(._.)", "(>_<)", "(*_*)", "(¬_¬)", "ಠ_ಠ", "ಠ︵ಠ", "(ಠ_ಠ)", "¯\\(ツ)/¯", "(╯°□°）╯︵┻━┻", "><(((*>",
        ];

        for orth in emoticons {
            m.insert(orth.to_string(), vec![ExceptionToken { orth: orth.to_string(), norm: None }]);
        }

        // --- Degree sign + C/F/K followed by a dot: ("°c.", "°F.", etc.) split into 3 tokens
        for u in &['c', 'f', 'k', 'C', 'F', 'K'] {
            let key = format!("°{}.", u);
            m.insert(
                key.clone(),
                vec![
                    ExceptionToken { orth: "°".into(), norm: None },
                    ExceptionToken { orth: u.to_string(), norm: None },
                    ExceptionToken { orth: ".".into(), norm: None },
                ],
            );
        }

        m
    };
}
// ...continuation from PART 2...

/// Retrieve exceptions for a given orth string, if any.
/// Returns a slice of ExceptionToken (or None if no exception).
pub fn get_exception(orth: &str) -> Option<&Vec<ExceptionToken>> {
    BASE_EXCEPTIONS.get(orth)
}

/// Example: checking if a word should be split as a special exception
///
/// ```rust
/// use mynlp::tokenizer_exceptions::{get_exception};
///
/// fn main() {
///     let s = "C++";
///     if let Some(excs) = get_exception(s) {
///         println!("Special exception for '{}': {:?}", s, excs);
///     }
/// }
/// ```
///
/// This will print:
/// Special exception for 'C++': [ExceptionToken { orth: "C++", norm: None }]
///
/// If you want to add language-specific exceptions at runtime:
pub fn merge_exceptions(custom: &[(String, Vec<ExceptionToken>)]) -> ExceptionMap {
    let mut new_map = BASE_EXCEPTIONS.clone();
    for (k, v) in custom.iter() {
        new_map.insert(k.clone(), v.clone());
    }
    new_map
}

/// You can then use the merged exception map instead of BASE_EXCEPTIONS.

/// ---------
/// # Summary
/// - Use `get_exception(orth)` to look up split rules for a given orthographic string.
/// - The ExceptionToken gives the desired split (with optional NORM value).
/// - You can extend the map for multi-lingual support.
/// - Used in your tokenizer pipeline to override normal tokenization for these forms.
/// - URL regex (`url_match()`) is also provided for matching URLs.
/// ---------

// End of tokenizer_exceptions.rs
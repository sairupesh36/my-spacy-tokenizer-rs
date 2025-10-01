// src/pattern.rs

#![allow(dead_code)] // Allow unused items for now

use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy; // Import Lazy for static initializations

// ----- CONSTANTS & TYPE ALIASES -----
pub const ORTH: &str = "ORTH";
pub const NORM: &str = "NORM";

pub type ExceptionAttributeMap = HashMap<String, String>;
pub type ExceptionTokenList = Vec<ExceptionAttributeMap>;
pub type ExceptionMap = HashMap<String, ExceptionTokenList>;

pub static EMOTICONS: &[&str] = &[
    ":)", ":-)", ":))", ":-))", ":)))", ":-)))", "(:", "(-:", "=)", "(=", ":]", ":-]", "[:", "[-:", "[=", "=]",
    ":o)", "(o:", ":}", ":-}", "8)", "8-)", "(-8", ";)", ";-)", "(;", "(-;", ":(", ":-(", ":((", ":-((", ":(((", ":-(((",
    "):", ")-:", "=(", ">:(", ":')", ":'-)", ":'(", ":'-(", ":/", ":-/", "=/", "=|", ":|", ":-|", "]=", "=[", ":1",
    ":P", ":-P", ":p", ":-p", ":O", ":-O", ":o", ":-o", ":0", ":-0", ":()", ">:o", ":*", ":-*", ":3", ":-3", "=3",
    ":>", ":->", ":X", ":-X", ":x", ":-x", ":D", ":-D", ";D", ";-D", "=D", "xD", "XD", "xDD", "XDD", "8D", "8-D",
    "^_^", "^__^", "^___^", ">.<", ">.>", "<.<", "._.", ";_;", "-_-", "-__-", "v.v", "V.V", "v_v", "V_V", "o_o",
    "o_O", "O_o", "O_O", "0_o", "o_0", "0_0", "o.O", "O.o", "O.O", "o.o", "0.0", "o.0", "0.o", "@_@", "<3", "<33",
    "<333", "</3", "(^_^)", "(-_-)", "(._.)", "(>_<)", "(*_*)", "(¬_¬)", "ಠ_ಠ", "ಠ︵ಠ", "(ಠ_ಠ)", "¯\\(ツ)/¯",
    "(╯°□°）╯︵┻━┻", "><(((*>",
];

// ----- HELPER FUNCTIONS -----
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn exc_entry(attrs: &[(&str, &str)]) -> ExceptionAttributeMap {
    let mut map = HashMap::new();
    for (k, v) in attrs.iter() {
        map.insert((*k).to_string(), (*v).to_string());
    }
    map
}

// ----- REGEX CHAR CLASSES -----
pub const FINAL_ALPHA_LOWER_CONTENT_STR: &str = "a-z";
pub const FINAL_ALPHA_UPPER_CONTENT_STR: &str = "A-Z";
pub const FINAL_ALPHA_CONTENT_STR: &str = "a-zA-Z";
pub const DIGITS_CONTENT_STR: &str = "0-9";
pub const FINAL_ALPHANUM_CONTENT_STR: &str = "a-zA-Z0-9";
pub const CONCAT_QUOTES_CONTENT_STR: &str = r#"'"`‘’“”„»«「」『』（）〔〕【】《》〈〉⟦⟧"#;

// Regex pattern parts
// For HYPHENS_PATTERN_PART, we'll split literals from multi-char regex parts
pub const SIMPLE_LITERAL_HYPHENS: &[&str] = &["-", "–", "—", "~"];
pub const REGEX_MULTI_HYPHENS_PART: &str = r"--|---|——"; // Regex part for multi-character hyphens

pub const LIST_ELLIPSES_LITERALS: &[&str] = &["…", "⋯", "⋮"];
pub const LIST_ELLIPSES_REGEX: &[&str] = &[r"\.{3,}", r"\.{2}"]; // Regex for 3+ dots and 2 dots

pub const LIST_ICONS_PATTERNS: &[&str] = &[ // These are assumed to be regex patterns
    r"[❤⭐👍✔✘]",
    r"[😊😂😍🤔😅]",
    r":placeholdericon1:",
    r"placeholdericon2",
];
pub const CURRENCY_PATTERN_PART: &str = r"\$|£|€|¥|฿|US\$|C\$|A\$|₽|﷼|₴|₠|₡|₢|₣|₤|₥|₦|₧|₨|₩|₪|₫|€|₭|₮|₯|₰|₱|₲|₳|₴|₵|₶|₷|₸|₹|₺|₻|₼|₽|₾|₿";
pub const UNITS_PATTERN_PART: &str = "km|km²|km³|m|m²|m³|dm|dm²|dm³|cm|cm²|cm³|mm|mm²|mm³|ha|µm|nm|yd|in|ft|kg|g|mg|µg|t|lb|oz|m/s|km/h|kmh|mph|hPa|Pa|mbar|mb|MB|kb|KB|gb|GB|tb|TB|T|G|M|K|%";

// ----- EXCEPTION GENERATION -----
fn get_abbreviations_list() -> Vec<&'static str> {
    vec![
        "'d", "a.m.", "Adm.", "Bros.", "co.", "Co.", "Corp.", "D.C.", "Dr.",
        "e.g.", "E.g.", "E.G.", "etc.", "Gen.", "Gov.", "i.e.", "I.e.", "I.E.",
        "Inc.", "Jr.", "Ltd.", "Md.", "Messrs.", "Mo.", "Mont.", "Mr.", "Mrs.",
        "Ms.", "p.m.", "Ph.D.", "Prof.", "Rep.", "Rev.", "Sen.", "Sr.", "St.",
        "vs.", "v.s.", "viz.", "U.S.", "U.K.", "N.Y.", "L.A.",
        "Dec.", "approx.",
    ]
}
static EXCLUDE_FROM_EXCEPTIONS_PY: &[&str] = &[
    "Ill", "ill", "Its", "its", "Hell", "hell", "Shell", "shell",
    "Shed", "shed", "were", "Were", "Well", "well", "Whore", "whore",
];
fn get_english_tokenizer_exceptions_inner() -> ExceptionMap {
    let mut exc: ExceptionMap = HashMap::new();

    // Pronouns (Translated from Python snippet)
    for pron_base in ["i"] {
        for orth_base in [pron_base, &capitalize(pron_base)] {
            exc.insert(
                format!("{}'m", orth_base),
                vec![
                    exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]),
                    exc_entry(&[(ORTH, "'m"), (NORM, "am")]),
                ],
            );
            exc.insert(
                format!("{}m", orth_base),
                vec![
                    exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]),
                    exc_entry(&[(ORTH, "m")]),
                ],
            );
            exc.insert(
                format!("{}'ma", orth_base),
                vec![
                    exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]),
                    exc_entry(&[(ORTH, "'m"), (NORM, "am")]),
                    exc_entry(&[(ORTH, "a"), (NORM, "gonna")]),
                ],
            );
            exc.insert(
                format!("{}ma", orth_base),
                vec![
                    exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]),
                    exc_entry(&[(ORTH, "m"), (NORM, "am")]),
                    exc_entry(&[(ORTH, "a"), (NORM, "gonna")]),
                ],
            );
        }
    }

    for pron_base in ["i", "you", "he", "she", "it", "we", "they"] {
        for orth_base in [pron_base, &capitalize(pron_base)] {
            exc.insert(format!("{}'ll", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'ll"), (NORM, "will")])]);
            exc.insert(format!("{}ll", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "ll"), (NORM, "will")])]);
            exc.insert(format!("{}'ll've", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'ll"), (NORM, "will")]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}llve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "ll"), (NORM, "will")]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
            exc.insert(format!("{}'d", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'d"), (NORM, "'d")])]);
            exc.insert(format!("{}d", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "d"), (NORM, "'d")])]);
            exc.insert(format!("{}'d've", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'d"), (NORM, "would")]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}dve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "d"), (NORM, "would")]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
        }
    }

    for pron_base in ["i", "you", "we", "they"] {
        for orth_base in [pron_base, &capitalize(pron_base)] {
            exc.insert(format!("{}'ve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}ve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
        }
    }

    for pron_base in ["you", "we", "they"] {
        for orth_base in [pron_base, &capitalize(pron_base)] {
            exc.insert(format!("{}'re", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'re"), (NORM, "are")])]);
            exc.insert(format!("{}re", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "re"), (NORM, "are")])]);
        }
    }

    for pron_base in ["he", "she", "it"] {
        for orth_base in [pron_base, &capitalize(pron_base)] {
            exc.insert(format!("{}'s", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "'s"), (NORM, "'s")])]);
            exc.insert(format!("{}s", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, pron_base)]), exc_entry(&[(ORTH, "s")])]);
        }
    }

    let w_words_data = [
        ("who", None), ("what", None), ("when", None), ("where", None), ("why", None),
        ("how", None), ("there", None), ("that", Some("Number=Sing|Person=3")),
        ("this", Some("Number=Sing|Person=3")), ("these", Some("Number=Plur|Person=3")),
        ("those", Some("Number=Plur|Person=3")),
    ];
    for (word_base, morph_str_opt) in &w_words_data {
        for orth_base in [*word_base, &capitalize(word_base)] {
            if *morph_str_opt != Some("Number=Plur|Person=3") {
                exc.insert(format!("{}'s", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'s"), (NORM, "'s")])]);
                exc.insert(format!("{}s", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "s")])]);
            }
            exc.insert(format!("{}'ll", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'ll"), (NORM, "will")])]);
            exc.insert(format!("{}ll", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "ll"), (NORM, "will")])]);
            exc.insert(format!("{}'ll've", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'ll"), (NORM, "will")]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}llve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "ll"), (NORM, "will")]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
            if *morph_str_opt != Some("Number=Sing|Person=3") {
                exc.insert(format!("{}'re", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'re"), (NORM, "are")])]);
                exc.insert(format!("{}re", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "re"), (NORM, "are")])]);
                exc.insert(format!("{}'ve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
                exc.insert(format!("{}ve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
            }
            exc.insert(format!("{}'d", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'d"), (NORM, "'d")])]);
            exc.insert(format!("{}d", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "d"), (NORM, "'d")])]);
            exc.insert(format!("{}'d've", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "'d"), (NORM, "would")]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}dve", orth_base), vec![exc_entry(&[(ORTH, orth_base), (NORM, word_base)]), exc_entry(&[(ORTH, "d"), (NORM, "would")]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
        }
    }

    let verbs_data_list1 = [
        ("ca", "can"), ("could", "could"), ("do", "do"),
        ("does", "does"), ("did", "do"), ("had", "have"),
        ("may", "may"), ("might", "might"), ("must", "must"),
        ("need", "need"), ("ought", "ought"), ("sha", "shall"),
        ("should", "should"), ("wo", "will"), ("would", "would"),
    ];
    for &(verb_orth_base, verb_norm_base) in &verbs_data_list1 {
        let capitalized_orth = capitalize(verb_orth_base);
        let orth_forms_to_process = [
            (verb_orth_base, verb_norm_base),
            (capitalized_orth.as_str(), verb_norm_base),
        ];
        for (current_orth, current_norm) in orth_forms_to_process.iter().cloned() {
            let first_token_attrs = exc_entry(&[(ORTH, current_orth), (NORM, current_norm)]);
            exc.insert(format!("{}n't", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "n't"), (NORM, "not")])]);
            exc.insert(format!("{}nt", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "nt"), (NORM, "not")])]);
            exc.insert(format!("{}n't've", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "n't"), (NORM, "not")]), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}ntve", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "nt"), (NORM, "not")]), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
        }
    }

    let verbs_data_list2 = [
        ("could", "could"), ("might", "might"), ("must", "must"),
        ("should", "should"), ("would", "would"),
    ];
    for &(verb_orth_base, verb_norm_base) in &verbs_data_list2 {
        let capitalized_orth = capitalize(verb_orth_base);
        let orth_forms_to_process = [
            (verb_orth_base, verb_norm_base),
            (capitalized_orth.as_str(), verb_norm_base),
        ];
        for (current_orth, current_norm) in orth_forms_to_process.iter().cloned() {
            let first_token_attrs = exc_entry(&[(ORTH, current_orth), (NORM, current_norm)]);
            exc.insert(format!("{}'ve", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "'ve"), (NORM, "have")])]);
            exc.insert(format!("{}ve", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "ve"), (NORM, "have")])]);
        }
    }

    let verbs_data_list3 = [
        ("ai", "ai"), ("are", "are"), ("is", "is"), ("was", "was"), ("were", "were"),
        ("have", "have"), ("has", "has"), ("dare", "dare"),
    ];
    for &(verb_orth_base, verb_norm_base) in &verbs_data_list3 {
        let capitalized_orth = capitalize(verb_orth_base);
        let orth_forms_to_process = [
            (verb_orth_base, verb_norm_base),
            (capitalized_orth.as_str(), verb_norm_base),
        ];
        for (current_orth, current_norm) in orth_forms_to_process.iter().cloned() {
            let first_token_attrs = exc_entry(&[(ORTH, current_orth), (NORM, current_norm)]);
            exc.insert(format!("{}n't", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "n't"), (NORM, "not")])]);
            exc.insert(format!("{}nt", current_orth), vec![first_token_attrs.clone(), exc_entry(&[(ORTH, "nt"), (NORM, "not")])]);
        }
    }

    let trailing_apos_exc_data = [
        ("doin", "doing"), ("goin", "going"), ("nothin", "nothing"),
        ("nuthin", "nothing"), ("ol", "old"), ("somethin", "something"),
    ];
    for &(orth_val, norm_val) in &trailing_apos_exc_data {
        for data_orth_case in [orth_val, &capitalize(orth_val)] {
            let base_entry = exc_entry(&[(ORTH, data_orth_case), (NORM, norm_val)]);
            let apos_orth = format!("{}'", data_orth_case);
            let apos_entry = exc_entry(&[(ORTH, &apos_orth), (NORM, norm_val)]);
            exc.insert(data_orth_case.to_string(), vec![base_entry.clone()]);
            exc.insert(apos_orth.clone(), vec![apos_entry.clone()]);
        }
    }

    let leading_apos_exc_data = [
        ("em", "them"), ("ll", "will"), ("nuff", "enough"),
    ];
    for &(orth_val, norm_val) in &leading_apos_exc_data {
        let base_entry = exc_entry(&[(ORTH, orth_val), (NORM, norm_val)]);
        let apos_orth = format!("'{}", orth_val);
        let apos_entry = exc_entry(&[(ORTH, &apos_orth), (NORM, norm_val)]);
        exc.insert(orth_val.to_string(), vec![base_entry]);
        exc.insert(apos_orth, vec![apos_entry]);
    }

    for h in 1..=12 {
        for period_variant in ["a.m.", "am"] {
            exc.insert(format!("{}{}", h, period_variant), vec![exc_entry(&[(ORTH, &h.to_string())]), exc_entry(&[(ORTH, period_variant), (NORM, "a.m.")])]);
        }
        for period_variant in ["p.m.", "pm"] {
            exc.insert(format!("{}{}", h, period_variant), vec![exc_entry(&[(ORTH, &h.to_string())]), exc_entry(&[(ORTH, period_variant), (NORM, "p.m.")])]);
        }
    }

    let other_exc_map_data: Vec<(&str, Vec<(&str, Option<&str>)>)> = vec![
        ("y'all", vec![("y'", Some("you")), ("all", None)]),
        ("yall", vec![("y", Some("you")), ("all", None)]),
        ("how'd'y", vec![("how", None), ("'d", None), ("'y", Some("you"))]),
        ("How'd'y", vec![("How", Some("how")), ("'d", None), ("'y", Some("you"))]),
        ("not've", vec![("not", None), ("'ve", Some("have"))]),
        ("notve", vec![("not", None), ("ve", Some("have"))]),
        ("Not've", vec![("Not", Some("not")), ("'ve", Some("have"))]),
        ("Notve", vec![("Not", Some("not")), ("ve", Some("have"))]),
        ("cannot", vec![("can", None), ("not", None)]),
        ("Cannot", vec![("Can", Some("can")), ("not", None)]),
        ("gonna", vec![("gon", Some("going")), ("na", Some("to"))]),
        ("Gonna", vec![("Gon", Some("going")), ("na", Some("to"))]),
        ("gotta", vec![("got", None), ("ta", Some("to"))]),
        ("Gotta", vec![("Got", Some("got")), ("ta", Some("to"))]),
        ("let's", vec![("let", None), ("'s", Some("us"))]),
        ("Let's", vec![("Let", Some("let")), ("'s", Some("us"))]),
        ("c'mon", vec![("c'm", Some("come")), ("on", None)]),
        ("C'mon", vec![("C'm", Some("come")), ("on", None)]),
    ];
    for (key, parts) in other_exc_map_data {
        let mut token_list = Vec::new();
        for (orth, norm_opt) in parts {
            if let Some(norm) = norm_opt {
                token_list.push(exc_entry(&[(ORTH, orth), (NORM, norm)]));
            } else {
                token_list.push(exc_entry(&[(ORTH, orth)]));
            }
        }
        exc.insert(key.to_string(), token_list);
    }

    let single_token_exceptions_data = [
        ("'S", Some("'s")), ("'s", Some("'s")), ("\u{2018}S", Some("'s")), ("\u{2018}s", Some("'s")),
        ("and/or", None), ("w/o", Some("without")), ("'re", Some("are")),
        ("'Cause", Some("because")), ("'cause", Some("because")), ("'cos", Some("because")),
        ("'Cos", Some("because")), ("'coz", Some("because")), ("'Coz", Some("because")),
        ("'cuz", Some("because")), ("'Cuz", Some("because")), ("'bout", Some("about")),
        ("ma'am", Some("madam")), ("Ma'am", Some("madam")),
        ("o'clock", None), ("O'clock", None),
        ("lovin'", Some("loving")), ("Lovin'", Some("loving")), ("lovin", Some("loving")), ("Lovin", Some("loving")),
        ("havin'", Some("having")), ("Havin'", Some("having")), ("havin", Some("having")), ("Havin", Some("having")),
        ("doin'", Some("doing")), ("Doin'", Some("doing")), ("doin", Some("doing")), ("Doin", Some("doing")),
        ("goin'", Some("going")), ("Goin'", Some("going")), ("goin", Some("going")), ("Goin", Some("going")),
        ("Mt.", Some("Mount")), ("Ak.", Some("Alaska")), ("Ala.", Some("Alabama")), ("Apr.", Some("April")),
        ("Ariz.", Some("Arizona")), ("Ark.", Some("Arkansas")), ("Aug.", Some("August")),
        ("Calif.", Some("California")), ("Colo.", Some("Colorado")), ("Conn.", Some("Connecticut")),
        ("Dec.", Some("December")), ("Del.", Some("Delaware")), ("Feb.", Some("February")),
        ("Fla.", Some("Florida")), ("Ga.", Some("Georgia")), ("Ia.", Some("Iowa")),
        ("Id.", Some("Idaho")), ("Ill.", Some("Illinois")), ("Ind.", Some("Indiana")),
        ("Jan.", Some("January")), ("Jul.", Some("July")), ("Jun.", Some("June")),
        ("Kan.", Some("Kansas")), ("Kans.", Some("Kansas")), ("Ky.", Some("Kentucky")),
        ("La.", Some("Louisiana")), ("Mar.", Some("March")), ("Mass.", Some("Massachusetts")),
        ("Mich.", Some("Michigan")), ("Minn.", Some("Minnesota")), ("Miss.", Some("Mississippi")),
        ("N.C.", Some("North Carolina")), ("N.D.", Some("North Dakota")), ("N.H.", Some("New Hampshire")),
        ("N.J.", Some("New Jersey")), ("N.M.", Some("New Mexico")), ("N.Y.", Some("New York")),
        ("Neb.", Some("Nebraska")), ("Nebr.", Some("Nebraska")), ("Nev.", Some("Nevada")),
        ("Nov.", Some("November")), ("Oct.", Some("October")), ("Okla.", Some("Oklahoma")),
        ("Ore.", Some("Oregon")), ("Pa.", Some("Pennsylvania")), ("S.C.", Some("South Carolina")),
        ("Sep.", Some("September")), ("Sept.", Some("September")), ("Tenn.", Some("Tennessee")),
        ("Va.", Some("Virginia")), ("Wash.", Some("Washington")), ("Wis.", Some("Wisconsin")),
    ];
    for (orth_val, norm_opt) in single_token_exceptions_data {
        if let Some(norm_val) = norm_opt {
            exc.insert(orth_val.to_string(), vec![exc_entry(&[(ORTH, orth_val), (NORM, norm_val)])]);
        } else {
            exc.insert(orth_val.to_string(), vec![exc_entry(&[(ORTH, orth_val)])]);
        }
    }

    let simple_orth_exceptions = [
        "'d", "a.m.", "Adm.", "Bros.", "co.", "Co.", "Corp.", "D.C.", "Dr.",
        "e.g.", "E.g.", "E.G.", "Gen.", "Gov.", "i.e.", "I.e.", "I.E.", "Inc.", "Jr.",
        "Ltd.", "Md.", "Messrs.", "Mo.", "Mont.", "Mr.", "Mrs.", "Ms.", "p.m.",
        "Ph.D.", "Prof.", "Rep.", "Rev.", "Sen.", "St.", "vs.", "v.s.",
    ];
    for orth_val in simple_orth_exceptions {
        exc.insert(orth_val.to_string(), vec![exc_entry(&[(ORTH, orth_val)])]);
    }

    for &abbr_from_list in get_abbreviations_list().iter() {
        if !exc.contains_key(abbr_from_list) {
             exc.insert(abbr_from_list.to_string(), vec![exc_entry(&[(ORTH, abbr_from_list)])]);
        }
    }

    for &emoticon in EMOTICONS {
        exc.insert(emoticon.to_string(), vec![exc_entry(&[(ORTH, emoticon)])]);
    }

    for string_to_exclude in EXCLUDE_FROM_EXCEPTIONS_PY {
        exc.remove(*string_to_exclude);
    }
    exc
}
pub fn get_english_tokenizer_exceptions() -> ExceptionMap {
    get_english_tokenizer_exceptions_inner()
}


// ----- TOKENIZER RULE PATTERNS -----

// Use Lazy to compute this once and cache it
static EMOTICON_ALTERNATION_REGEX_STR: Lazy<String> = Lazy::new(|| {
    let mut sorted_emoticons: Vec<&str> = EMOTICONS.to_vec();
    sorted_emoticons.sort_by_key(|a| std::cmp::Reverse(a.len()));
    let escaped_emoticon_patterns: Vec<String> = sorted_emoticons
        .iter()
        .map(|&s| fancy_regex::escape(s).into_owned())
        .collect();
    format!("(?:{})", escaped_emoticon_patterns.join("|"))
});

// Helper for suffix patterns (and token_match if emoticons are there)
fn get_emoticon_alternation_regex_str() -> String {
    EMOTICON_ALTERNATION_REGEX_STR.clone()
}

pub fn get_english_prefix_patterns() -> Vec<&'static str> {
    // This list remains unchanged, as prefixes are usually single characters or simple.
    vec![
        r"§", r"%", r"=", r"—", r"–", r"\+(?![0-9])",
        r"\(", r"\[", r"\{", r"<",
        r#"""#, r"'", r"`", r"“", r"‘", r"‚", r"„", r"«", r"»",
        r"「", r"」", r"『", r"』", r"（", r"〔", r"【", r"《", r"〈", r"⟦",
        r"\$", r"¢", r"£", r"€", r"¥", r"֏", r"؋", r"₡", r"₢", r"₣", r"₤", r"₥", r"₦", r"₧",
        r"₨", r"₩", r"₪", r"₫", r"₭", r"₮", r"₯", r"₰", r"₱", r"₲", r"₳", r"₴", r"₵", r"₸",
        r"₺", r"₼", r"₽", r"₾", r"₿", r"៛", r"₹",
        r"#", r"&",
    ]
}

pub fn get_english_suffix_patterns() -> Vec<String> {
    // Suffixes can be complex, so they largely remain regex-based.
    let emoticon_regex_str = get_emoticon_alternation_regex_str();
    let mut patterns = vec![
        emoticon_regex_str,
    ];
    for s_regex in LIST_ELLIPSES_REGEX.iter() { // Use the regex ellipses
        patterns.push(s_regex.to_string());
    }

    // Use a Lazy static for pre-escaped literals
    static ESCAPED_ELLIPSES_LITERALS: Lazy<Vec<String>> = Lazy::new(|| {
        LIST_ELLIPSES_LITERALS.iter().map(|&s| fancy_regex::escape(s).into_owned()).collect()
    });
    patterns.extend(ESCAPED_ELLIPSES_LITERALS.iter().cloned());


    static ESCAPED_ABBREVS_LIST: Lazy<Vec<String>> = Lazy::new(|| {
        get_abbreviations_list().iter().map(|&s| fancy_regex::escape(s).into_owned()).collect()
    });
    patterns.extend(ESCAPED_ABBREVS_LIST.iter().cloned());

    let common_suffixes: Vec<String> = vec![
        r":".to_string(), r";".to_string(), r"!".to_string(), r"\?".to_string(), r"\.".to_string(), r",".to_string(),
        r"\)".to_string(), r"\]".to_string(), r"\}".to_string(), r">".to_string(),
        r#"""#.to_string(), r"'".to_string(), r"`".to_string(), r"”".to_string(), r"’".to_string(), r"‚".to_string(), r"„".to_string(), r"»".to_string(), r"«".to_string(),
        r"」".to_string(), r"「".to_string(), r"』".to_string(), r"『".to_string(), r"）".to_string(), r"〕".to_string(), r"】".to_string(), r"》".to_string(), r"〉".to_string(), r"⟧".to_string(),
        r"'s".to_string(), r"'S".to_string(), r"’s".to_string(), r"’S".to_string(),
        r"—".to_string(), r"–".to_string(), // These are single char, but often part of suffix rules
        r"(?<=[0-9])\+".to_string(),
        r"(?<=°[FfCcKk])\.".to_string(),
        format!(r"(?<=[0-9])(?:{})", CURRENCY_PATTERN_PART),
        format!(r"(?<=[0-9])(?:{})", UNITS_PATTERN_PART),
        format!(r"(?<=[{alphanum}%²\-+{quotes}])\.",
            alphanum = FINAL_ALPHANUM_CONTENT_STR,
            quotes = CONCAT_QUOTES_CONTENT_STR.replace('[', r"\[").replace(']', r"\]").replace('-', r"\-")
        ),
        r"(?<=[A-Z][A-Z])\.".to_string(),
    ];
    patterns.extend(common_suffixes);
    patterns
}


// NEW: Function to get only literal infix strings for AhoCorasick
pub fn get_english_literal_infix_strings() -> Vec<String> {
    let mut literals: Vec<String> = Vec::new();

    // Emoticons (these are great candidates for Aho-Corasick)
    // Exclude certain emoticons that might cause issues with LeftmostFirst (e.g. subsets)
    let emoticons_to_exclude: HashSet<&str> =
        vec!["o.o", "0.0", "._.", ":0", ":1", ":3"].into_iter().collect();
    for &emoticon_str in EMOTICONS.iter() {
        if !emoticons_to_exclude.contains(emoticon_str) {
            literals.push(emoticon_str.to_string());
        }
    }

    // Simple Parentheses and Brackets
    literals.extend([
        "(".to_string(), ")".to_string(), "[".to_string(), "]".to_string(),
        "{".to_string(), "}".to_string(), "<".to_string(), ">".to_string(),
    ]);

    // Simple Literal Hyphens
    for &hyphen in SIMPLE_LITERAL_HYPHENS.iter() {
        literals.push(hyphen.to_string());
    }

    // Literal Ellipses
    for &ellipsis in LIST_ELLIPSES_LITERALS.iter() {
        literals.push(ellipsis.to_string());
    }

    // Add other simple literal strings that were previously part of infix regexes.
    // For example, if ":" or "/" were treated as infixes and are always literal.
    literals.push(":".to_string());
    literals.push("/".to_string());
    literals.push("=".to_string());

    // Sort by length descending to help LeftmostFirst behavior if patterns can overlap
    // For AhoCorasick's MatchKind::LeftmostFirst, the order of patterns in the input vector matters
    // if there are multiple matches starting at the same position. The *first* pattern
    // encountered in the input list that matches is chosen. Sorting by length ensures
    // longer matches are preferred if they start at the same point as shorter ones.
    literals.sort_by_key(|b| std::cmp::Reverse(b.len()));

    // Deduplicate, just in case (sorting may have put duplicates next to each other)
    let mut unique_literals = Vec::new();
    let mut seen = HashSet::new();
    for lit in literals {
        if seen.insert(lit.clone()) {
            unique_literals.push(lit);
        }
    }
    unique_literals
}

// MODIFIED: This function now returns only the true REGEX infix patterns
pub fn get_english_regex_infix_patterns() -> Vec<String> {
    let mut patterns = vec![
        // Regex-based Ellipses
        LIST_ELLIPSES_REGEX[0].to_string(), // r"\.{3,}"
        LIST_ELLIPSES_REGEX[1].to_string(), // r"\.{2}"

        // Complex Infixes with lookarounds or specific contexts
        // Splits "1+2", "1-2", "1*2", "1^2"
        format!(r"(?<=[{digits}])[+\-*^](?=[{digits}-])", digits = DIGITS_CONTENT_STR),
        // Splits "A.B", "a.B" if it's not an abbreviation (e.g., U.S.A.)
        // This regex means: match a dot if it's preceded by a lowercase or quote char AND followed by an uppercase or quote char.
        // This is a common spaCy rule to split e.g. "St.Louis" into "St." and "Louis"
        format!(r"(?<=[{lower}{quotes}])\.(?=[{upper}{quotes}])",
                lower = FINAL_ALPHA_LOWER_CONTENT_STR,
                upper = FINAL_ALPHA_UPPER_CONTENT_STR,
                quotes = CONCAT_QUOTES_CONTENT_STR.replace('[', r"\[").replace(']', r"\]").replace('-', r"\-")),
        // Splits "word,word" (e.g. "hello,world")
        format!(r"(?<=[{alpha}]),(?=[{alpha}])", alpha = FINAL_ALPHA_CONTENT_STR),

        // Regex for multi-character hyphens (if not handled as literals)
        REGEX_MULTI_HYPHENS_PART.to_string(), // e.g., r"--|---|——"

        // Complex infixes for things like "word:/word" or "word=/word" or "word<=word"
        // (but not if : or = are part of a URL)
        // This splits characters like ':', '<', '>', '=', '/' when they are within alphanumeric sequences.
        format!(r"(?<=[{alphanum}])[:<>=/](?=[{alpha}])",
                alphanum = FINAL_ALPHANUM_CONTENT_STR,
                alpha = FINAL_ALPHA_CONTENT_STR),
        format!(r"(?<=[{alpha}])[:<>=/](?=[{alphanum}])",
                alphanum = FINAL_ALPHANUM_CONTENT_STR,
                alpha = FINAL_ALPHA_CONTENT_STR),
    ];

    // Icon patterns (assuming these are regexes)
    for icon_pattern in LIST_ICONS_PATTERNS.iter() {
        patterns.push(icon_pattern.to_string());
    }

    patterns
}


pub fn get_english_token_match_pattern_str() -> Option<String> {
    // This function remains largely the same.
    // Ensure emoticons here are handled via get_emoticon_alternation_regex_str
    // or individually escaped if that's how it was.
    let mut token_patterns: Vec<String> = vec![
        // Numbers with currency, decimals, commas
        format!(r"(?:{})[0-9]{{1,3}}(?:,[0-9]{{3}})*(?:\.[0-9]{{2}})?", CURRENCY_PATTERN_PART),
        format!(r"(?:{})[0-9]+(?:\.[0-9]{{2}})?", CURRENCY_PATTERN_PART),
        r"[+-]?\d+\.\d{2}".to_string(), // e.g., +1.00, -0.50
        r"[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?".to_string(), // e.g., 1,000,000 or 1.23
        r"[+-]?\d+\.\d+".to_string(), // e.g., 1.234
        r"[+-]?\.\d+".to_string(), // e.g., .50
        r"[+-]?\d+".to_string(), // e.g., 123
    ];

    // Add regex ellipses to token_match
    for s_regex in LIST_ELLIPSES_REGEX.iter() {
        token_patterns.push(s_regex.to_string());
    }
    // Add literal ellipses (escaped for regex) to token_match
    static ESCAPED_ELLIPSES_LITERALS_FOR_TOKEN_MATCH: Lazy<Vec<String>> = Lazy::new(|| {
        LIST_ELLIPSES_LITERALS.iter().map(|&s| fancy_regex::escape(s).into_owned()).collect()
    });
    token_patterns.extend(ESCAPED_ELLIPSES_LITERALS_FOR_TOKEN_MATCH.iter().cloned());

    token_patterns.extend([
        r"[%]".to_string(),
        r"[°ºª]".to_string(),
        r"&(?:amp|lt|gt|quot|apos);".to_string(), // HTML entities
        r"[®©™℠]".to_string(), // Registered, Copyright, Trademark symbols
    ]);

    static ESCAPED_ABBREVS_LIST_FOR_TOKEN_MATCH: Lazy<Vec<String>> = Lazy::new(|| {
        get_abbreviations_list().iter().map(|&s| fancy_regex::escape(s).into_owned()).collect()
    });
    token_patterns.extend(ESCAPED_ABBREVS_LIST_FOR_TOKEN_MATCH.iter().cloned());


    // Add emoticons as a single alternation pattern
    token_patterns.push(get_emoticon_alternation_regex_str());

    for icon_pattern_str in LIST_ICONS_PATTERNS.iter() {
        token_patterns.push(icon_pattern_str.to_string());
    }
    Some(format!(r"^(?:{})$", token_patterns.join("|")))
}

pub fn get_english_url_match_pattern_str() -> String {
    // This function remains unchanged.
    let alpha_lower_chars = FINAL_ALPHA_LOWER_CONTENT_STR;
    let pattern_parts: Vec<String> = vec![
        r"^".to_string(),
        r"(?:(?:[\w+\-.]{2,})://)?".to_string(),
        r"(?:\S+(?::\S*)?@)?".to_string(),
        r"(?:".to_string(),
            r"(?!(?:10|127)(?:\.\d{1,3}){3})".to_string(),
            r"(?!(?:169\.254|192\.168)(?:\.\d{1,3}){2})".to_string(),
            r"(?!172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})".to_string(),
            r"(?:[1-9]\d?|1\d\d|2[01]\d|22[0-3])".to_string(),
            r"(?:\.(?:1?\d{1,2}|2[0-4]\d|25[0-5])){2}".to_string(),
            r"(?:\.(?:[1-9]\d?|1\d\d|2[0-4]\d|25[0-4]))".to_string(),
        r"|".to_string(),
            r"(?:".to_string(),
                r"(?:".to_string(),
                    r"[A-Za-z0-9\u00a1-\uffff]".to_string(),
                    r"[A-Za-z0-9\u00a1-\uffff_-]{0,62}".to_string(),
                r")?".to_string(),
                r"[A-Za-z0-9\u00a1-\uffff]\.".to_string(),
            r")+" .to_string(),
            format!(r"(?:[{}]{{2,63}})", alpha_lower_chars),
        r")".to_string(),
        r"(?::\d{2,5})?".to_string(),
        r"(?:[/?#]\S*)?".to_string(),
        r"$".to_string()
    ];
    pattern_parts.join("")
}
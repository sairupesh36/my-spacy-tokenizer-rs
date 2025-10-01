// src/main.rs
use fancy_regex::Regex;
use std::collections::HashSet; // Still used in pattern.rs for literal infix exclusions
use std::fs;
use std::env;
use std::time::Instant;
use std::sync::Arc; // For Arc to share rules across threads

// Add AhoCorasick imports
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

// Add rayon for parallel processing
use rayon::prelude::*; // Import parallel iterators

mod pattern;
use pattern::{ExceptionMap, ORTH};

// Set to false for optimal performance in release builds.
// Set to true for debugging logic with detailed print statements.
const ENABLE_DEBUG_PRINTING: bool = false;

struct TokenizerRules {
    prefixes: Vec<Regex>,
    suffixes: Vec<Regex>,
    regex_infixes: Vec<Regex>,
    literal_infix_matcher: Option<AhoCorasick>,
    token_match: Option<Regex>,
    url_match: Option<Regex>,
    exceptions: ExceptionMap,
}

// Implement Sync and Send for TokenizerRules if its members are Sync/Send
// Regex, AhoCorasick, Vec, HashMap are all Sync + Send, so this derive works.
unsafe impl Sync for TokenizerRules {}
unsafe impl Send for TokenizerRules {}

impl TokenizerRules {
    fn new() -> Self {
        let prefixes = pattern::get_english_prefix_patterns()
            .iter()
            .map(|s| Regex::new(s).unwrap_or_else(|e| panic!("Prefix compile error for pattern '{}': {}", s, e)))
            .collect();

        let suffixes = pattern::get_english_suffix_patterns()
            .into_iter()
            .map(|s_string| Regex::new(&s_string).unwrap_or_else(|e| panic!("Suffix compile error for pattern '{}': {}", s_string, e)))
            .collect();

        let regex_infixes = pattern::get_english_regex_infix_patterns()
            .into_iter()
            .map(|s_string| Regex::new(&s_string).unwrap_or_else(|e| panic!("Regex Infix compile error for pattern '{}': {}", s_string, e)))
            .collect();

        let literal_infix_strings = pattern::get_english_literal_infix_strings();
        let literal_infix_matcher = if !literal_infix_strings.is_empty() {
            Some(
                AhoCorasickBuilder::new()
                    // LeftmostFirst is often what you want for tokenizers,
                    // as it matches the first pattern defined if there are overlaps.
                    // Combined with sorting `literal_infix_strings` by length descending,
                    // it ensures longest matches are preferred if they start at the same point.
                    .match_kind(MatchKind::LeftmostFirst)
                    .build(&literal_infix_strings)
                    .unwrap_or_else(|e| panic!("AhoCorasick build error: {}", e)),
            )
        } else {
            None
        };

        let token_match_str_opt = pattern::get_english_token_match_pattern_str();
        let token_match = token_match_str_opt.map(|s| Regex::new(&s).expect("Invalid token_match regex"));

        let url_match_str = pattern::get_english_url_match_pattern_str();
        let url_match = Some(Regex::new(&url_match_str).unwrap_or_else(|e| panic!("Invalid url_match regex for pattern '{}': {}", url_match_str, e)));

        let exceptions = pattern::get_english_tokenizer_exceptions();

        TokenizerRules {
            prefixes,
            suffixes,
            regex_infixes,
            literal_infix_matcher,
            token_match,
            url_match,
            exceptions,
        }
    }
}


/// Tokenizes a single chunk of text, applying prefix, suffix, and infix rules.
/// Returns a vector of (token_text, start_char_offset, end_char_offset) tuples.
fn tokenize_chunk(
    original_chunk: &str,
    rules: &TokenizerRules,
    base_char_offset: usize, // Base character offset of this chunk within the original text
) -> Vec<(String, usize, usize)> {
    if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Processing chunk: '{}' (base_offset: {})", original_chunk, base_char_offset); }
    let mut tokens_with_offsets: Vec<(String, usize, usize)> = Vec::new();
    let chunk_char_count = original_chunk.chars().count();

    if original_chunk.is_empty() {
        if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Empty chunk, returning empty."); }
        return tokens_with_offsets;
    }

    // 1. Check for exact match in exceptions
    if let Some(exception_rules) = rules.exceptions.get(original_chunk) {
        if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Found exception for: '{}'", original_chunk); }
        let mut current_sub_offset_chars = 0;
        for token_attrs_map in exception_rules {
            if let Some(orth_val_str) = token_attrs_map.get(ORTH) {
                let token_text = orth_val_str.clone();
                let token_char_len = token_text.chars().count();
                tokens_with_offsets.push((
                    token_text,
                    base_char_offset + current_sub_offset_chars,
                    base_char_offset + current_sub_offset_chars + token_char_len,
                ));
                current_sub_offset_chars += token_char_len;
            }
        }
        // Verify if the exception fully consumes the chunk
        let total_exception_chars: usize = tokens_with_offsets.iter().map(|(s, _, _)| s.chars().count()).sum();
        if total_exception_chars == chunk_char_count {
            if ENABLE_DEBUG_PRINTING {
                println!("  [tokenize_chunk] Exception fully matched chunk. Returning: {:?}", tokens_with_offsets.iter().map(|(s,_,_)|s.as_str()).collect::<Vec<&str>>());
            }
            return tokens_with_offsets;
        } else {
            // If exception doesn't fully cover, treat it as not an exact match and continue processing
            if ENABLE_DEBUG_PRINTING {
                println!("  [tokenize_chunk] Warning: Exception for '{}' (len {}) did not cover whole chunk (exception toks len {}). Falling through to standard tokenization.",
                    original_chunk, chunk_char_count, total_exception_chars);
            }
            tokens_with_offsets.clear(); // Clear partial tokens from exception
        }
    }

    // 2. Check for token_match (e.g., numbers, single-token emoticons, specific symbols)
    // This catches entire chunks that should be single tokens
    if let Some(re) = &rules.token_match {
        if let Ok(Some(mat)) = re.find(original_chunk) {
            if mat.start() == 0 && mat.end() == original_chunk.len() {
                if ENABLE_DEBUG_PRINTING {
                    println!("  [tokenize_chunk] Matched token_match: '{}'", original_chunk);
                }
                return vec![(original_chunk.to_string(), base_char_offset, base_char_offset + chunk_char_count)];
            }
        }
    }

    // 3. Check for url_match if the chunk is a URL
    if let Some(re) = &rules.url_match {
        if let Ok(Some(mat)) = re.find(original_chunk) {
            if mat.start() == 0 && mat.end() == original_chunk.len() {
                if ENABLE_DEBUG_PRINTING {
                    println!("  [tokenize_chunk] Matched url_match: '{}'", original_chunk);
                }
                return vec![(original_chunk.to_string(), base_char_offset, base_char_offset + chunk_char_count)];
            }
        }
    }

    // If not handled by exceptions, token_match, or URL_match, proceed with splitting
    let mut current_work_slice = original_chunk; // The current portion of the chunk being processed
    let mut current_relative_char_offset_in_chunk = 0; // Offset within the original_chunk

    // --- Prefix Stripping ---
    if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Starting prefix stripping for: '{}'", current_work_slice); }
    loop {
        if current_work_slice.is_empty() { break; }
        let mut matched_this_iteration = false;
        for re_prefix in &rules.prefixes {
            // Find the longest, leftmost match
            // `find` method finds the first match; since prefixes are always at the start, this is sufficient.
            match re_prefix.find(current_work_slice) {
                Ok(Some(mat)) if mat.start() == 0 && !mat.as_str().is_empty() => {
                    let prefix_text = mat.as_str().to_string();
                    if ENABLE_DEBUG_PRINTING { println!("    [tokenize_chunk] Found prefix: '{}'", prefix_text); }
                    let prefix_char_len = prefix_text.chars().count();
                    tokens_with_offsets.push((
                        prefix_text,
                        base_char_offset + current_relative_char_offset_in_chunk,
                        base_char_offset + current_relative_char_offset_in_chunk + prefix_char_len,
                    ));
                    current_relative_char_offset_in_chunk += prefix_char_len;
                    current_work_slice = &current_work_slice[mat.end()..]; // Slice the string for remaining work
                    matched_this_iteration = true;
                    break;
                }
                _ => {}
            }
        }
        if !matched_this_iteration { break; } // No more prefixes matched
    }
    if ENABLE_DEBUG_PRINTING {
        println!("  [tokenize_chunk] After prefixes, remaining for suffix/infix: '{}', tokens so far: {:?}", current_work_slice, tokens_with_offsets.iter().map(|(s,_,_)|s.as_str()).collect::<Vec<&str>>());
    }

    // --- Suffix Stripping ---
    let mut suffixes_found_reversed: Vec<String> = Vec::new(); // Store suffixes to add them at the end
    if !current_work_slice.is_empty() {
        if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Starting suffix stripping for: '{}'", current_work_slice); }
        loop {
            let mut matched_this_iteration = false;
            for re_suffix in &rules.suffixes {
                // Find all matches, then pick the rightmost longest one
                if let Ok(matches) = re_suffix.find_iter(current_work_slice).collect::<Result<Vec<_>, _>>() {
                    // Find the rightmost match that ends at the end of the current slice
                    if let Some(mat) = matches.into_iter().rev().find(|m| m.end() == current_work_slice.len() && !m.as_str().is_empty()) {
                        let suffix_text = mat.as_str().to_string();
                        if ENABLE_DEBUG_PRINTING { println!("    [tokenize_chunk] Found suffix: '{}'", suffix_text); }
                        suffixes_found_reversed.push(suffix_text);
                        current_work_slice = &current_work_slice[..mat.start()]; // Slice from the start up to the suffix
                        matched_this_iteration = true;
                        break;
                    }
                }
            }
            if !matched_this_iteration || current_work_slice.is_empty() { break; } // No more suffixes matched
        }
    }
    if ENABLE_DEBUG_PRINTING {
        println!("  [tokenize_chunk] After suffixes, remaining for infix: '{}', suffixes found (reversed): {:?}", current_work_slice, suffixes_found_reversed);
    }

    // --- Infix Tokenization ---
    if !current_work_slice.is_empty() {
        if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Applying infixes to: '{}'", current_work_slice); }
        let infix_parts = simple_infix_tokenize_chunk_internal(
            current_work_slice,
            rules.literal_infix_matcher.as_ref(),
            &rules.regex_infixes
        );
        if ENABLE_DEBUG_PRINTING { println!("    [tokenize_chunk] Infix parts: {:?}", infix_parts); }
        let mut infix_part_char_offset_in_chunk = current_relative_char_offset_in_chunk;
        for part in infix_parts {
            let part_char_len = part.chars().count();
            tokens_with_offsets.push((
                part,
                base_char_offset + infix_part_char_offset_in_chunk,
                base_char_offset + infix_part_char_offset_in_chunk + part_char_len,
            ));
            infix_part_char_offset_in_chunk += part_char_len;
        }
        current_relative_char_offset_in_chunk = infix_part_char_offset_in_chunk;
    }

    // --- Re-attach Suffixes ---
    for suffix_text in suffixes_found_reversed.into_iter().rev() { // Re-reverse to original order
        let suffix_char_len = suffix_text.chars().count();
        tokens_with_offsets.push((
            suffix_text,
            base_char_offset + current_relative_char_offset_in_chunk,
            base_char_offset + current_relative_char_offset_in_chunk + suffix_char_len,
        ));
        current_relative_char_offset_in_chunk += suffix_char_len;
    }

    // --- Fallback: If no splitting happened, take the whole chunk ---
    if tokens_with_offsets.is_empty() && !original_chunk.is_empty() {
        if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] No rules split chunk, taking original chunk: '{}'", original_chunk); }
        tokens_with_offsets.push((
            original_chunk.to_string(),
            base_char_offset,
            base_char_offset + chunk_char_count,
        ));
    }

    if ENABLE_DEBUG_PRINTING { println!("  [tokenize_chunk] Final tokens for chunk '{}': {:?}", original_chunk, tokens_with_offsets.iter().map(|(s,_,_)|s.as_str()).collect::<Vec<&str>>()); }
    tokens_with_offsets
}

/// Internal helper for infix tokenization within a given string slice.
/// Returns a vector of strings representing the infix-tokenized parts.
fn simple_infix_tokenize_chunk_internal(
    chunk: &str,
    literal_matcher: Option<&AhoCorasick>,
    regex_infixes: &[Regex],
) -> Vec<String> {
    if ENABLE_DEBUG_PRINTING { println!("    [infix_internal] Processing: '{}'", chunk); }
    if chunk.is_empty() { return Vec::new(); }

    let mut all_found_infix_spans: Vec<(usize, usize)> = Vec::new(); // (byte_start, byte_end)

    // 1. Find matches with AhoCorasick (literal infixes)
    if let Some(matcher) = literal_matcher {
        for mat in matcher.find_iter(chunk) {
            if (mat.end() - mat.start()) > 0 {
                if ENABLE_DEBUG_PRINTING {
                    println!("      [infix_internal] AhoCorasick matched pattern ('{}') at bytes {}-{}",
                        chunk[mat.start()..mat.end()].to_string(), mat.start(), mat.end()
                    );
                }
                all_found_infix_spans.push((mat.start(), mat.end()));
            }
        }
    }

    // 2. Find matches with fancy-regex (regex infixes)
    for (pattern_idx, re) in regex_infixes.iter().enumerate() {
        if let Ok(iter_matches) = re.find_iter(chunk).collect::<Result<Vec<_>, _>>() {
            for mat in iter_matches {
                if !mat.as_str().is_empty() {
                    if ENABLE_DEBUG_PRINTING {
                        println!("      [infix_internal] Regex Infix Pattern #{} ('{}') matched: '{}' at bytes {}-{}", pattern_idx, re.as_str(), mat.as_str(), mat.start(), mat.end());
                    }
                    all_found_infix_spans.push((mat.start(), mat.end()));
                }
            }
        }
    }

    if all_found_infix_spans.is_empty() {
        if ENABLE_DEBUG_PRINTING { println!("    [infix_internal] No infixes (literal or regex) found in '{}'. Returning as whole.", chunk); }
        return vec![chunk.to_string()];
    }

    // 3. Combine, Sort, and Filter Overlapping Matches
    // Sort by start position, then by length (longer first) to correctly handle overlaps
    // This mimics spaCy's preference for longest match at the same starting point.
    all_found_infix_spans.sort_by_key(|k| (k.0, std::cmp::Reverse(k.1 - k.0)));

    let mut filtered_matches: Vec<(usize, usize)> = Vec::new();
    let mut current_processed_byte_end = 0;
    for &(byte_start, byte_end) in &all_found_infix_spans {
        if byte_start >= current_processed_byte_end {
            // New match, no overlap or past previous match
            filtered_matches.push((byte_start, byte_end));
            current_processed_byte_end = byte_end;
        } else if byte_end > current_processed_byte_end {
            // Overlap: If new match extends further, update end of last match.
            // This is a simplified merge. spaCy has more complex rules for merging.
            let last_match_idx = filtered_matches.len() - 1;
            filtered_matches[last_match_idx].1 = byte_end;
            current_processed_byte_end = byte_end;
        }
    }
    // Re-sort by start position (should mostly be sorted already due to previous sort and merge logic)
    filtered_matches.sort_by_key(|k| k.0);

    if ENABLE_DEBUG_PRINTING {
        println!("    [infix_internal] Filtered combined infix matches for '{}': {:?}", chunk, filtered_matches);
    }

    // 4. Split chunk into tokens based on filtered_matches
    let mut tokens: Vec<String> = Vec::new();
    let mut last_byte_end = 0;

    for (byte_start, byte_end) in filtered_matches {
        // Add text before the infix match
        if byte_start > last_byte_end {
            let part = chunk[last_byte_end..byte_start].to_string();
            if !part.is_empty() { tokens.push(part); }
        }
        // Add the infix match itself
        let infix_part = chunk[byte_start..byte_end].to_string();
        tokens.push(infix_part);
        last_byte_end = byte_end;
    }

    // Add any remaining text after the last infix match
    if last_byte_end < chunk.len() {
        let final_part = chunk[last_byte_end..].to_string();
        if !final_part.is_empty() { tokens.push(final_part); }
    }

    // Fallback: If no tokens were produced but the chunk was not empty, add the whole chunk
    if tokens.is_empty() && !chunk.is_empty() {
        tokens.push(chunk.to_string());
    }

    if ENABLE_DEBUG_PRINTING {
        println!("    [infix_internal] Tokens for '{}': {:?}", chunk, tokens);
    }
    tokens
}


/// Tokenizes a single sentence string in parallel by splitting it into whitespace-separated chunks,
/// tokenizing those chunks, and then reassembling the results.
///
/// Returns a vector of the final tokens for the sentence.
fn advanced_tokenize_sentence_parallel(
    sentence: &str,
    rules: &Arc<TokenizerRules>, // Shared reference to tokenizer rules
    original_sentence_char_offset: usize, // Start character offset of this sentence in the whole text
) -> Vec<String> {
    if ENABLE_DEBUG_PRINTING { println!("[advanced_tokenize_sentence_parallel] Original Sentence for splitting: '{}'", sentence); }

    // Collect whitespace-separated chunks with their character-based offsets within the sentence.
    // This allows accurate base_char_offset calculation for `tokenize_chunk`.
    let mut chunks_info: Vec<(usize, &str)> = Vec::new(); // (char_offset_in_sentence, chunk_text)
    let mut current_byte_offset = 0; // Current byte offset within the sentence slice
    let mut current_char_offset_in_sentence = 0; // Current char offset within the sentence slice

    for chunk_str_raw in sentence.split_whitespace() {
        // Find the actual byte start of the current word chunk in the sentence
        let byte_start_of_chunk = sentence[current_byte_offset..].find(chunk_str_raw)
            .map(|relative_byte_offset| current_byte_offset + relative_byte_offset)
            .unwrap_or_else(|| panic!("Failed to find word chunk in sentence slice: '{}' in '{}'", chunk_str_raw, &sentence[current_byte_offset..]));

        // Account for any whitespace *before* this chunk
        let whitespace_len_bytes = byte_start_of_chunk - current_byte_offset;
        current_char_offset_in_sentence += sentence[current_byte_offset..byte_start_of_chunk].chars().count();

        chunks_info.push((current_char_offset_in_sentence, chunk_str_raw));

        // Update offsets for the next iteration
        current_char_offset_in_sentence += chunk_str_raw.chars().count();
        current_byte_offset = byte_start_of_chunk + chunk_str_raw.len();
    }

    // Parallel processing of chunks within this sentence.
    // Each chunk is tokenized by `tokenize_chunk`, and the result is a vector of tokens.
    // We maintain the `char_offset_in_sentence` so we can sort the results correctly.
    let tokenized_chunks_unordered: Vec<(usize, Vec<String>)> = chunks_info.into_par_iter()
        .map(|(chunk_char_offset_in_sentence, chunk_str)| {
            // Calculate the absolute character offset for this chunk in the entire text
            let base_char_offset_for_chunk = original_sentence_char_offset + chunk_char_offset_in_sentence;
            let tokens_with_offsets = tokenize_chunk(chunk_str, rules, base_char_offset_for_chunk);

            // Extract just the token strings
            let tokens_only: Vec<String> = tokens_with_offsets.into_iter().map(|(s, _, _)| s).collect();
            (chunk_char_offset_in_sentence, tokens_only) // Return original relative char offset and tokens
        })
        .collect(); // Collect results back into a Vec (order is not guaranteed here)

    // Sort the results by their original relative character offset within the sentence
    let mut sorted_tokenized_chunks = tokenized_chunks_unordered;
    sorted_tokenized_chunks.sort_by_key(|(offset, _)| *offset);

    // Flatten the Vec<(offset, Vec<String>)> into a single Vec<String>
    let mut final_tokens_for_sentence: Vec<String> = Vec::new();
    for (_, tokens_list) in sorted_tokenized_chunks {
        final_tokens_for_sentence.extend(tokens_list);
    }

    if ENABLE_DEBUG_PRINTING {
        let total_token_chars: usize = final_tokens_for_sentence.iter().map(|t| t.chars().count()).sum();
        let sentence_chars: usize = sentence.chars().count();
        // This check is very sensitive. It might fail if a rule drops characters,
        // or if leading/trailing whitespace isn't explicitly handled in `tokenize_chunk`'s offsets.
        // It's a good debug check, but not always a strict requirement depending on tokenizer spec.
        if total_token_chars != sentence_chars {
             println!("[advanced_tokenize_sentence_parallel] WARNING: Character count mismatch for sentence '{}'! Expected: {}, Got: {}. This might indicate issues with offset tracking or whitespace handling in `tokenize_chunk`.", sentence, sentence_chars, total_token_chars);
             println!("[advanced_tokenize_sentence_parallel] Final tokens: {:?}", final_tokens_for_sentence);
        }
    }
    final_tokens_for_sentence
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("--- spaCy-like English Tokenizer (Rust Demo) ---");
        eprintln!("Usage: {} <filename>", args[0]);
        eprintln!("Please provide a text file to tokenize.");
        std::process::exit(1);
    }
    let filename = &args[1];

    // Configure Rayon thread pool (optional, but good for explicit control)
    // Rayon by default tries to use all available logical cores.
    // Setting it explicitly ensures it uses the specified number of threads.
    // If you have 24 logical cores, rayon will likely use 24 by default anyway.
    rayon::ThreadPoolBuilder::new().num_threads(24).build_global().unwrap();


    println!("--- spaCy-like English Tokenizer (Rust Demo) ---");
    let rules_init_start = Instant::now();
    let rules = TokenizerRules::new();
    let rules_init_duration = rules_init_start.elapsed();
    println!("Tokenizer rules initialized. (Took {:?})", rules_init_duration);

    if ENABLE_DEBUG_PRINTING {
        println!("  Loaded {} prefix patterns.", rules.prefixes.len());
        println!("  Loaded {} suffix patterns.", rules.suffixes.len());
        println!("  Loaded {} regex infix patterns.", rules.regex_infixes.len());
        if rules.literal_infix_matcher.is_some() {
            let literal_count = pattern::get_english_literal_infix_strings().len(); // Re-calling to get count
            println!("  Loaded {} literal infix patterns for AhoCorasick.", literal_count);
        } else {
            println!("  No literal infix patterns loaded for AhoCorasick.");
        }
        println!("  Loaded {} exception entries.", rules.exceptions.len());
    }

    println!("\nProcessing file: {}", filename);
    let content_read_start = Instant::now();
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            std::process::exit(1);
        }
    };
    let content_read_duration = content_read_start.elapsed();
    println!("Content read from file. (Took {:?})", content_read_duration);


    if content.is_empty() {
        println!("File is empty. No tokens to process.");
        return;
    }
    if content.trim().is_empty() {
        println!("File contains only whitespace. No word tokens to process.");
    }

    // Share rules across threads using Arc
    let rules_arc = Arc::new(rules);

    let start_time = Instant::now();

    // Split content into lines. Each line will be processed in parallel.
    // For very large text files that are not naturally line-delimited sentences,
    // you might need a more robust sentence segmenter here (e.g., using a regex
    // for sentence-ending punctuation like `.!?` followed by a space).
    // However, for `dedup.txt` which often has one sentence per line, this is adequate.
    let lines: Vec<&str> = content.lines().collect();

    let mut all_tokens: Vec<String> = Vec::new();
    let mut current_global_char_offset = 0;

    // Process lines (as sentences) sequentially, but allow internal chunking to be parallel.
    // The loop is sequential to easily track `current_global_char_offset` and
    // handle newlines correctly. If sentence splitting itself could be parallelized
    // (and results sorted), that would be an even further optimization.
    for line in lines {
        let tokens_for_line = advanced_tokenize_sentence_parallel(
            line,
            &rules_arc,
            current_global_char_offset
        );
        all_tokens.extend(tokens_for_line);
        current_global_char_offset += line.chars().count();
        // Account for the newline character itself in the offset, if it exists (for non-empty lines)
        if !line.is_empty() || content.ends_with('\n') { // Check if it's a real newline
            current_global_char_offset += 1; // For the '\n' character
        }
    }


    let duration = start_time.elapsed();

    println!("\nTime taken to tokenize: {:?}", duration);
    println!("Total tokens produced: {}", all_tokens.len());

    println!("\nSample of first 20 tokens (or all if fewer):");
    let num_tokens_to_show = all_tokens.len().min(20);
    if num_tokens_to_show == 0 {
        println!("(No tokens produced from file content)");
    } else {
        let sample_tokens_slice = &all_tokens[0..num_tokens_to_show];
        println!("{}", sample_tokens_slice.join(" | "));
    }

    if ENABLE_DEBUG_PRINTING && !all_tokens.is_empty() {
        println!("\n--- Full Individual Token List ({} tokens total): ---", all_tokens.len());
        for (i, token) in all_tokens.iter().enumerate() {
            println!("{:3}: \"{}\"", i + 1, token);
        }
    }
    println!("\n--- Tokenization complete ---");
}
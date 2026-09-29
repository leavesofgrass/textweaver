//! English function words, left out of the sentence vectors.
//!
//! The list is textweaver's own: articles, pronouns, auxiliary and modal
//! verbs, prepositions, conjunctions, and the commonest adverbs, written
//! the way the tokenizer leaves them (lowercase, apostrophes dropped, a
//! final `'s` already removed). It is kept short on purpose: SCOWL's
//! smallest size lists everyday content words too ("house", "water"), so it
//! would throw away the words a summary is about (ADR-0037). Inverse
//! document frequency weighs down whatever common words remain, in any
//! language.

/// The stop words, sorted, for a binary search.
#[rustfmt::skip]
const WORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "all", "almost", "also", "although",
    "always", "am", "among", "an", "and", "another", "any", "are", "arent", "around", "as", "at",
    "be", "because", "been", "before", "being", "below", "between", "both", "but", "by", "can",
    "cannot", "cant", "could", "couldnt", "did", "didnt", "do", "does", "doesnt", "doing", "done",
    "dont", "down", "during", "each", "either", "else", "enough", "etc", "even", "ever", "every",
    "few", "for", "from", "further", "had", "hadnt", "has", "hasnt", "have", "havent", "having",
    "he", "hed", "hell", "her", "here", "hers", "herself", "hes", "him", "himself", "his", "how",
    "however", "i", "id", "if", "ill", "im", "in", "into", "is", "isnt", "it", "itll", "its",
    "itself", "ive", "just", "least", "less", "let", "lets", "like", "may", "maybe", "me",
    "might", "mine", "more", "most", "much", "must", "mustnt", "my", "myself", "neither", "never",
    "no", "nor", "not", "now", "of", "off", "often", "on", "once", "one", "only", "onto", "or",
    "other", "others", "otherwise", "ought", "our", "ours", "ourselves", "out", "over", "own",
    "per", "perhaps", "quite", "rather", "really", "same", "shall", "shant", "she", "shed",
    "shell", "shes", "should", "shouldnt", "since", "so", "some", "such", "than", "that",
    "thats", "the", "their", "theirs", "them", "themselves", "then", "there", "therefore",
    "theres", "these", "they", "theyd", "theyll", "theyre", "theyve", "this", "those", "though",
    "through", "thus", "to", "too", "toward", "towards", "under", "until", "up", "upon", "us",
    "very", "via", "was", "wasnt", "we", "wed", "well", "were", "werent", "weve", "what",
    "whats", "when", "where", "whether", "which", "while", "who", "whom", "whos", "whose", "why",
    "will", "with", "within", "without", "wont", "would", "wouldnt", "yet", "you", "youd",
    "youll", "your", "youre", "yours", "yourself", "yourselves", "youve",
];

/// True when `word` (lowercase, as the tokenizer leaves it) is a stop word.
pub fn is_stop_word(word: &str) -> bool {
    WORDS.binary_search(&word).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_sorted_and_unique() {
        for pair in WORDS.windows(2) {
            assert!(pair[0] < pair[1], "{:?} before {:?}", pair[0], pair[1]);
        }
    }

    #[test]
    fn finds_function_words_and_not_content_words() {
        for w in ["the", "and", "dont", "youre", "however"] {
            assert!(is_stop_word(w), "{w}");
        }
        for w in ["reader", "house", "water", "summary"] {
            assert!(!is_stop_word(w), "{w}");
        }
    }
}

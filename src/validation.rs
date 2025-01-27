pub fn is_valid_word(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }

    // Iterate over each character and ensure it is alphabetic
    input.chars().all(|c| c.is_alphabetic() && c.is_lowercase())
}

/// Interprets a user input string as a boolean value.
///
/// In regex notation, maps strings of type `no?/i` into `false`,
/// and `^y(es)?/i` into `true`. Strings not matching these patterns
/// are mapped into the `no_match_default` value.
///
/// Input strings are trimmed at the start, so surrounding whitespace
/// should not affect parsing.
pub fn user_str_into_bool(user_input: &str, no_match_default: bool) -> bool {
    let user_input = user_input.trim().to_ascii_lowercase();
    match user_input.len() {
        0 => no_match_default,
        1 => match user_input.chars().next().unwrap() {
            'y' => true,
            'n' => false,
            _ => no_match_default,
        },
        2 => {
            if user_input == "no" {
                false
            } else {
                no_match_default
            }
        }
        3 => {
            if user_input == "yes" {
                true
            } else {
                no_match_default
            }
        }
        _ => no_match_default,
    }
}

pub trait SentinelPlugin: Send + Sync {
    fn name(&self) -> &str;
    // Updated to return (MethodName, ErrorMessage) for better Ruby-side context
    fn check(&self, content: &str) -> Vec<(String, String)>;
}

pub struct VoidArgumentPlugin;

impl SentinelPlugin for VoidArgumentPlugin {
    fn name(&self) -> &str {
        "Void Argument"
    }
    fn check(&self, content: &str) -> Vec<(String, String)> {
        let mut issues = Vec::new();
        let mut current_method = String::from("unknown");

        for line in content.lines() {
            if line.trim().starts_with("def ") {
                current_method = line
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .replace("def ", "")
                    .trim()
                    .to_string();
            }

            if line.contains(": void ->") || line.contains("(void)") {
                issues.push((
                    current_method.clone(),
                    "Uses 'void' as an argument (use '()' instead)".to_string(),
                ));
            }
        }
        issues
    }
}

pub struct AngleBracketPlugin;

impl SentinelPlugin for AngleBracketPlugin {
    fn name(&self) -> &str {
        "Angle Bracket"
    }
    fn check(&self, content: &str) -> Vec<(String, String)> {
        let mut issues = Vec::new();
        let mut current_method = String::from("top-level");

        for line in content.lines() {
            if line.trim().starts_with("def ") {
                current_method = line
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .replace("def ", "")
                    .trim()
                    .to_string();
            }

            // Skip comment lines
            if line.trim().starts_with('#') {
                continue;
            }

            // Match patterns like Array<X>, Hash<X>, Set<X>, etc.
            // Look for a capitalized identifier followed by <
            let bytes = line.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'<' && i > 0 {
                    // Check if preceded by an identifier char (letter/digit/underscore)
                    let prev = bytes[i - 1];
                    if prev.is_ascii_alphanumeric() || prev == b'_' {
                        // Walk back to find the start of the identifier
                        let mut start = i - 1;
                        while start > 0
                            && (bytes[start - 1].is_ascii_alphanumeric()
                                || bytes[start - 1] == b'_'
                                || bytes[start - 1] == b':')
                        {
                            start -= 1;
                        }
                        let ident = &line[start..i];
                        // Only flag if the identifier starts with uppercase (a type name)
                        if ident
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase() || c == ':')
                        {
                            issues.push((
                                current_method.clone(),
                                format!(
                                    "'{}<...>' uses angle brackets. RBS uses square brackets: '{}[...]'",
                                    ident, ident
                                ),
                            ));
                            break; // One issue per line is enough
                        }
                    }
                }
            }
        }
        issues
    }
}

#[cfg(test)]
mod angle_bracket_tests {
    use super::*;

    fn check(input: &str) -> Vec<(String, String)> {
        AngleBracketPlugin.check(input)
    }

    #[test]
    fn catches_array_angle() {
        let issues = check("  def foo: () -> Array<Hash>");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].1.contains("Array"));
    }

    #[test]
    fn catches_hash_angle() {
        let issues = check("  def bar: (Hash<String, Integer>) -> void");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].1.contains("Hash"));
    }

    #[test]
    fn ignores_square_brackets() {
        let issues = check("  def foo: () -> Array[Hash[untyped, untyped]]");
        assert!(issues.is_empty());
    }

    #[test]
    fn ignores_class_inheritance() {
        // class Foo < Bar should not trigger
        let issues = check("class Foo < ApplicationRecord");
        assert!(issues.is_empty());
    }

    #[test]
    fn ignores_comments() {
        let issues = check("# @return Array<Hash>");
        assert!(issues.is_empty());
    }

    #[test]
    fn tracks_method_name() {
        let input = "  def my_method: () -> Array<String>";
        let issues = check(input);
        assert_eq!(issues[0].0, "my_method");
    }
}

pub struct TypeCasePlugin;

impl SentinelPlugin for TypeCasePlugin {
    fn name(&self) -> &str {
        "Type Case"
    }
    fn check(&self, content: &str) -> Vec<(String, String)> {
        let mut issues = Vec::new();
        let mut current_method = String::from("top-level");

        // The "Wall of Shame" for lowercase primitives
        let primitives = ["string", "integer", "boolean", "array", "hash"];

        for line in content.lines() {
            // Track the method context so the user knows where to look
            if line.trim().starts_with("def ") {
                current_method = line
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .replace("def ", "")
                    .trim()
                    .to_string();
            }

            let bytes = line.as_bytes();

            for p in primitives {
                let mut search_from = 0;
                while let Some(rel) = line[search_from..].find(p) {
                    let idx = search_from + rel;
                    let after = idx + p.len();
                    search_from = after;

                    // Word boundaries: not part of a longer identifier
                    // (e.g. "string_helper" should not match "string").
                    let boundary_before = idx == 0
                        || !(bytes[idx - 1].is_ascii_alphanumeric() || bytes[idx - 1] == b'_');
                    let boundary_after = bytes
                        .get(after)
                        .is_none_or(|&c| !(c.is_ascii_alphanumeric() || c == b'_'));
                    if !boundary_before || !boundary_after {
                        continue;
                    }

                    // A lowercase word immediately followed by ':' is a
                    // keyword-arg *name* (e.g. the `array` in `array: Array[String]`),
                    // not a type — skip it.
                    if bytes.get(after) == Some(&b':') {
                        continue;
                    }

                    // Only flag where a type is actually expected: right after
                    // `(`, `,`, `[`, `:`, or `->` (skipping intervening spaces).
                    // This is what catches Sentinel's own named-positional-arg
                    // style (`(string paramName, ...)`), not just the bracketed
                    // and keyword-value forms the original patterns covered.
                    let mut before = idx;
                    while before > 0 && bytes[before - 1] == b' ' {
                        before -= 1;
                    }
                    let in_type_position =
                        before == 0 || matches!(bytes[before - 1], b'(' | b',' | b'[' | b':' | b'>');

                    if in_type_position {
                        issues.push((
                            current_method.clone(),
                            format!("Found lowercase type '{}'. RBS requires 'String', 'Integer', 'Array', etc.", p)
                        ));
                        break;
                    }
                }
            }
        }
        issues
    }
}

#[cfg(test)]
mod type_case_tests {
    use super::*;

    fn check(input: &str) -> Vec<(String, String)> {
        TypeCasePlugin.check(input)
    }

    #[test]
    fn catches_bare_single_arg() {
        let issues = check("  def foo: (string) -> void");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].1.contains("string"));
    }

    #[test]
    fn catches_keyword_arg_value() {
        let issues = check("  def foo: (name: string) -> void");
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn catches_generic_arg() {
        let issues = check("  def foo: () -> Array[string]");
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn catches_return_type() {
        let issues = check("  def foo: () -> string");
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn catches_named_positional_arg_first() {
        // Sentinel's own "Type paramName" positional-arg convention.
        let issues = check("  def create_user: (string username, String password) -> bool");
        assert_eq!(issues.len(), 1, "got: {:?}", issues);
        assert!(issues[0].1.contains("string"));
    }

    #[test]
    fn catches_named_positional_arg_after_comma() {
        let issues = check("  def create_user: (String username, string password) -> bool");
        assert_eq!(issues.len(), 1, "got: {:?}", issues);
    }

    #[test]
    fn ignores_lowercase_keyword_arg_name() {
        // `array` here is a keyword-arg *name*, not a type.
        let issues = check("  def foo: (array: Array[String]) -> void");
        assert!(issues.is_empty(), "got: {:?}", issues);
    }

    #[test]
    fn ignores_correct_case() {
        let issues = check("  def create_user: (String username, String password) -> bool");
        assert!(issues.is_empty());
    }

    #[test]
    fn ignores_substring_identifiers() {
        let issues = check("  def foo: (string_helper: String) -> void");
        assert!(issues.is_empty(), "got: {:?}", issues);
    }

    #[test]
    fn tracks_method_name() {
        let issues = check("  def my_method: (string) -> void");
        assert_eq!(issues[0].0, "my_method");
    }
}

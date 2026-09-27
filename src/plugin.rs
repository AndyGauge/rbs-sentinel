/// The contract every lint plugin implements.
///
/// To add a plugin: copy one of the structs below as a starting point,
/// `impl SentinelPlugin for YourPlugin`, then add a `Plugin::YourPlugin(YourPlugin)`
/// variant and one match arm in each of `Plugin::name`/`Plugin::check`.
///
/// Plugins are dispatched through the `Plugin` enum below, not
/// `Box<dyn SentinelPlugin>`. The set is small, fixed, and known entirely at
/// compile time — nothing registers a plugin from outside this crate — so a
/// `match` costs nothing over a vtable call and, unlike `Vec<Box<dyn ...>>`,
/// never heap-allocates to hold what is, per plugin, a zero-sized type.
pub trait SentinelPlugin {
    fn name(&self) -> &str;
    // Updated to return (MethodName, ErrorMessage) for better Ruby-side context
    fn check(&self, content: &str) -> Vec<(String, String)>;
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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

/// Every built-in lint plugin, in the order `check`/`init`/`watch`/`lsp` run
/// them. `Copy`: each variant wraps a zero-sized unit struct, so `Plugin::ALL`
/// and every value of this type cost nothing to construct or pass around.
#[derive(Clone, Copy)]
pub enum Plugin {
    VoidArgument(VoidArgumentPlugin),
    AngleBracket(AngleBracketPlugin),
    TypeCase(TypeCasePlugin),
}

impl Plugin {
    pub const ALL: [Plugin; 3] = [
        Plugin::VoidArgument(VoidArgumentPlugin),
        Plugin::AngleBracket(AngleBracketPlugin),
        Plugin::TypeCase(TypeCasePlugin),
    ];

    pub fn name(&self) -> &str {
        match self {
            Plugin::VoidArgument(p) => p.name(),
            Plugin::AngleBracket(p) => p.name(),
            Plugin::TypeCase(p) => p.name(),
        }
    }

    pub fn check(&self, content: &str) -> Vec<(String, String)> {
        match self {
            Plugin::VoidArgument(p) => p.check(content),
            Plugin::AngleBracket(p) => p.check(content),
            Plugin::TypeCase(p) => p.check(content),
        }
    }
}

#[cfg(test)]
mod plugin_enum_tests {
    use super::*;

    #[test]
    fn plugin_is_tiny_and_stack_only() {
        // Confirms the zero-allocation claim: no heap, minimal footprint.
        assert!(std::mem::size_of::<Plugin>() <= 1);
        assert_eq!(std::mem::size_of::<[Plugin; 3]>(), std::mem::size_of::<Plugin>() * 3);
    }

    #[test]
    fn all_covers_every_plugin_by_name() {
        let names: Vec<&str> = Plugin::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["Void Argument", "Angle Bracket", "Type Case"]);
    }
}

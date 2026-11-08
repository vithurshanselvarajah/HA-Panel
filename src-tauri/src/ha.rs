pub const TOKENS_KEY: &str = "hassTokens";

pub fn token_script(token: &str) -> String {
    let token_json = serde_json::to_string(token).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"(function () {{
  try {{
    var token = {token_json};
    if (!token) {{
      return;
    }}



    var tokens = {{
      access_token: token,
      token_type: "Bearer",
      expires_in: 1800,
      expires: Math.floor(Date.now() / 1000) + 1800,
      refresh_token: token,
      hassUrl: undefined
    }};
    window.localStorage.setItem("{TOKENS_KEY}", JSON.stringify(tokens));
  }} catch (err) {{

    console.warn("ha-panel: could not seed the Home Assistant token", err);
  }}
}})();
"#
    )
}

pub fn clear_token_script() -> String {
    format!(
        r#"(function () {{
  try {{
    window.localStorage.removeItem("{TOKENS_KEY}");
  }} catch (err) {{
    console.warn("ha-panel: could not clear the Home Assistant token", err);
  }}
}})();
"#
    )
}

pub fn kiosk_style_script() -> String {
    r#"(function () {
  try {
    var style = document.createElement("style");
    style.textContent = [
      "html, body {",
      "  overflow: hidden !important;",
      "  overscroll-behavior: contain !important;",
      "  -webkit-user-select: none !important;",
      "  user-select: none !important;",
      "  -webkit-tap-highlight-color: transparent !important;",
      "}",
      "* {",
      "  scrollbar-width: none !important;",
      "}",
      "::-webkit-scrollbar {",
      "  display: none !important;",
      "  width: 0 !important;",
      "  height: 0 !important;",
      "}",
      "::-webkit-scrollbar-thumb {",
      "  background: transparent !important;",
      "}"
    ].join("\n");
    (document.head || document.documentElement).appendChild(style);
  } catch (err) {
    console.warn("ha-panel: could not apply the kiosk styles", err);
  }
})();
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_script_writes_the_tokens_key() {
        let script = token_script("abc123");
        assert!(script.contains(TOKENS_KEY));
        assert!(script.contains("abc123"));
        assert!(script.contains("access_token"));
        assert!(script.contains("refresh_token"));
    }

    #[test]
    fn a_token_with_quotes_does_not_break_the_script() {
        let token = r#"a"b\c`d"#;
        let script = token_script(token);

        assert!(!script.contains(r#"a"b\c`d"#));
        assert!(script.contains(r#""a\"b\\c`d""#));
    }

    #[test]
    fn an_empty_token_produces_a_script_that_does_nothing() {
        let script = token_script("");
        assert!(script.contains("if (!token)"));
    }

    #[test]
    fn the_clear_script_removes_the_tokens_key() {
        let script = clear_token_script();
        assert!(script.contains("removeItem"));
        assert!(script.contains(TOKENS_KEY));
    }

    #[test]
    fn the_style_script_hides_the_scrollbars() {
        let script = kiosk_style_script();
        assert!(script.contains("scrollbar-width"));
        assert!(script.contains("user-select"));
    }
}

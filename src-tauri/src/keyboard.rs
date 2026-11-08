pub fn type_script(text: &str, backspace: bool) -> String {
    let text_json = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"(function () {{
  try {{
    var active = document.activeElement;
    if (!active) {{
      return;
    }}


    var tag = (active.tagName || "").toUpperCase();
    var editable = active.isContentEditable;
    if (!editable && tag !== "INPUT" && tag !== "TEXTAREA") {{
      return;
    }}
    if (!editable && active.readOnly) {{
      return;
    }}

    if ({backspace}) {{
      if (typeof active.selectionStart !== "number" || typeof active.selectionEnd !== "number") {{
        return;
      }}
      var start = active.selectionStart;
      var end = active.selectionEnd;
      if (start === end && start > 0) {{

        var value = active.value;
        var previous = value.slice(0, start - 1).search(/.$/);
        if (previous < 0) {{
          previous = 0;
        }} else {{
          previous = previous + 1;
        }}
        active.value = value.slice(0, previous) + value.slice(end);
        active.selectionStart = previous;
        active.selectionEnd = previous;
      }} else if (start !== end) {{
        var text = active.value;
        active.value = text.slice(0, start) + text.slice(end);
        active.selectionStart = start;
        active.selectionEnd = start;
      }}
    }} else {{
      var text = {text_json};
      var start = active.selectionStart;
      var end = active.selectionEnd;
      if (typeof start !== "number" || typeof end !== "number") {{
        active.value = active.value + text;
      }} else {{
        var value = active.value;
        active.value = value.slice(0, start) + text + value.slice(end);
        var caret = start + text.length;
        active.selectionStart = caret;
        active.selectionEnd = caret;
      }}
    }}


    active.dispatchEvent(new Event("input", {{ bubbles: true, composed: true }}));
    active.dispatchEvent(new Event("change", {{ bubbles: true, composed: true }}));
  }} catch (err) {{
    console.warn("ha-panel: could not type into the page", err);
  }}
}})();
"#
    )
}

pub fn focus_script() -> String {
    r#"(function () {
  try {
    var active = document.activeElement;
    if (!active) {
      return false;
    }
    var tag = (active.tagName || "").toUpperCase();
    if (active.isContentEditable) {
      return true;
    }
    return (tag === "INPUT" || tag === "TEXTAREA") && !active.readOnly;
  } catch (err) {
    return false;
  }
})();
"#
    .to_string()
}

pub fn watch_focus_script() -> String {
    r#"(function () {
  try {
    if (window.__haPanelFocusWatch) {
      return;
    }
    window.__haPanelFocusWatch = true;

    var report = function () {
      var active = document.activeElement;
      var focused = false;
      if (active) {
        var tag = (active.tagName || "").toUpperCase();
        focused =
          active.isContentEditable ||
          ((tag === "INPUT" || tag === "TEXTAREA") && !active.readOnly);
      }
      try {
        if (window.__TAURI_INTERNALS__) {
          window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
            event: "ha-panel://focus",
            payload: focused
          });
        }
      } catch (err) {

      }
    };

    document.addEventListener("focusin", report, true);
    document.addEventListener("focusout", function () {


      setTimeout(report, 50);
    }, true);
  } catch (err) {
    console.warn("ha-panel: could not watch for focus", err);
  }
})();
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_type_script_carries_the_character() {
        let script = type_script("a", false);
        assert!(script.contains("\"a\""));
        assert!(script.contains("input"));
    }

    #[test]
    fn a_character_with_a_quote_cannot_break_the_script() {
        let script = type_script(r#"a"b"#, false);

        assert!(script.contains(r#""a\"b""#));
    }

    #[test]
    fn the_backspace_script_takes_the_other_branch() {
        let script = type_script("", true);
        assert!(script.contains("selectionStart"));
        assert!(script.contains("slice(0, start - 1)"));
    }

    #[test]
    fn the_focus_script_looks_for_a_text_field() {
        let script = focus_script();
        assert!(script.contains("isContentEditable"));
        assert!(script.contains("TEXTAREA"));
    }

    #[test]
    fn the_watch_script_only_installs_itself_once() {
        let script = watch_focus_script();
        assert!(script.contains("__haPanelFocusWatch"));
        assert!(script.contains("focusin"));
    }
}
